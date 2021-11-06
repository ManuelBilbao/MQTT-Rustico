use crate::server::{make_connection, ClientFlags, PacketThings};
use std::io::{Read, Write};
use std::net::Shutdown;
use tracing::{debug, info};

const MQTT_VERSION: u8 = 4;
const MQTT_NAME: [u8; 6] = [0x00, 0x04, 0x4D, 0x51, 0x54, 0x54];
const _CONNECTION_IDENTIFIER_REFUSED: u8 = 2;
const CONNECTION_PROTOCOL_REJECTED: u8 = 1;
const INCORRECT_SERVER_CONNECTION: u8 = 1;
const SUCCESSFUL_CONNECTION: u8 = 0;

pub enum Packet {
    Connect,
    ConnAck,
    Publish,
    PubAck,
    PubRel,
    PubComp,
    Subscribe,
    SubAck,
    Unsubscribe,
    UnsubAck,
    PingReq,
    PingResp,
    Disconnect,
}

impl From<u8> for Packet {
    fn from(code: u8) -> Self {
        match code & 0xF0 {
            0x10 => Packet::Connect,
            0x20 => Packet::ConnAck,
            0x30 => Packet::Publish,
            0x40 => Packet::PubAck,
            0x60 => Packet::PubRel,
            0x70 => Packet::PubComp,
            0x80 => Packet::Subscribe,
            0x90 => Packet::SubAck,
            0xA0 => Packet::Unsubscribe,
            0xB0 => Packet::UnsubAck,
            0xC0 => Packet::PingReq,
            0xD0 => Packet::PingResp,
            0xE0 => Packet::Disconnect,
            _ => Packet::Disconnect,
        }
    }
}

impl From<Packet> for u8 {
    fn from(code: Packet) -> Self {
        match code {
            Packet::Connect => 0x10,
            Packet::ConnAck => 0x20,
            Packet::Publish => 0x30,
            Packet::PubAck => 0x40,
            Packet::PubRel => 0x62,
            Packet::PubComp => 0x70,
            Packet::Subscribe => 0x82,
            Packet::SubAck => 0x90,
            Packet::Unsubscribe => 0xA2,
            Packet::UnsubAck => 0xB0,
            Packet::PingReq => 0xC0,
            Packet::PingResp => 0xD0,
            Packet::Disconnect => 0xE0,
        }
    }
}

pub fn read_packet(
    client: &mut ClientFlags,
    packet_type: Packet,
    buffer_size: u8,
    byte_0: u8,
) -> Result<(), std::io::Error> {
    let mut buffer_packet: Vec<u8> = vec![0; buffer_size as usize];
    client.connection.read_exact(&mut buffer_packet)?;
    match packet_type {
        Packet::Connect => {
            println!("Connect package received");
            match make_connection(client, buffer_packet) {
                Ok(session_present) => {
                    send_connection_result(client, SUCCESSFUL_CONNECTION, session_present);
                    inform_client_id_to_coordinator(client);
                }
                Err(error_code) => {
                    send_connection_result(client, error_code, 0);
                }
            }
        }
        Packet::Publish => {
            println!("Received Publish package");
            match make_publication(client, buffer_packet, byte_0) {
                Ok(paquete_identifier) => {
                    send_publication_results(client, paquete_identifier);
                }
                Err(_) => {
                    println!("error when publishing");
                }
            }
        }
        Packet::Subscribe => {
            change_subscription(client, buffer_packet, Packet::Subscribe);
        }
        Packet::Unsubscribe => {
            change_subscription(client, buffer_packet, Packet::Unsubscribe);
        }
        Packet::PingReq => {
            send_pingresp(client);
        }
        Packet::Disconnect => {
            //TODO will message
            println!("Me llego un disconnect");
            close_stream(client);
        }
        _ => {
            println!("Received unknown package");
        }
    }

    Ok(())
}

fn close_stream(client: &mut ClientFlags) {
    let mut buffer = Vec::new();
    let aux = client.connection.read_to_end(&mut buffer);

    match aux {
        Ok(0) => {
            println!("El cliente ya habia cerrado el stream");
        }
        _ => {
            client
                .connection
                .shutdown(Shutdown::Both)
                .expect("shutdown call failed");
            println!("Cerre el stream con el cliente");
        }
    }
}

fn inform_client_id_to_coordinator(client: &mut ClientFlags) {
    if let Some(client_id) = &client.client_id {
        let mut bytes: Vec<u8> = client_id.as_bytes().to_vec();
        let mut buffer_packet: Vec<u8> = Vec::new();
        buffer_packet.append(&mut bytes);
        let packet_to_server = PacketThings {
            thread_id: client.id,
            packet_type: Packet::Connect,
            bytes: buffer_packet,
        };
        let sender = client.sender.lock();
        match sender {
            Ok(sender_ok) => {
                match sender_ok.send(packet_to_server) {
                    Ok(_) => {
                        println!("Success sending client id change to the coordinator thread")
                    }
                    Err(_) => {
                        println!("Error sending client id change to coordinator thread")
                    }
                };
            }
            Err(_) => {
                println!("Error reading coordinator channel")
            }
        }
    }
}

fn change_subscription(client: &mut ClientFlags, buffer_packet: Vec<u8>, tipo: Packet) {
    let packet_to_server = PacketThings {
        thread_id: client.id,
        packet_type: tipo,
        bytes: buffer_packet,
    };
    let sender = client.sender.lock();
    match sender {
        Ok(sender_ok) => {
            match sender_ok.send(packet_to_server) {
                Ok(_) => {
                    println!("Success sending subscription change to the coordinator thread")
                }
                Err(_) => {
                    println!("Error sending subscription change to coordinator thread")
                }
            };
        }
        Err(_) => {
            println!("Error reading subscription change")
        }
    }
}

pub fn verify_protocol_name(buffer: &[u8]) -> Result<(), u8> {
    for i in 0..6 {
        if buffer[i] != MQTT_NAME[i] {
            debug!("Wrong connection protocol name");
            return Err(INCORRECT_SERVER_CONNECTION);
        }
    }
    info!("Correct connection protocol name");
    Ok(())
}

pub fn verify_version_protocol(level: &u8) -> Result<(), u8> {
    if *level == MQTT_VERSION {
        return Ok(());
    }
    Err(CONNECTION_PROTOCOL_REJECTED)
}

fn send_connection_result(client: &mut ClientFlags, result_code: u8, session_present: u8) {
    let mut buffer = [0u8; 4];
    buffer[0] = Packet::ConnAck.into();
    buffer[1] = 0x02;
    buffer[2] = session_present;
    buffer[3] = result_code;

    client.connection.write_all(&buffer).unwrap();
    println!("Envié el connac");
}
fn send_publication_results(client: &mut ClientFlags, packet_identifier: [u8; 2]) {
    let mut buffer = [0u8; 4];
    buffer[0] = Packet::PubAck.into();
    buffer[1] = 0x02;
    buffer[2] = packet_identifier[0];
    buffer[3] = packet_identifier[1];

    client.connection.write_all(&buffer).unwrap();
    println!("Envié el puback");
}

fn make_publication(
    client: &mut ClientFlags,
    mut buffer_packet: Vec<u8>,
    byte_0: u8,
) -> Result<[u8; 2], String> {
    let topic_size: usize = ((buffer_packet[0] as usize) << 8) + buffer_packet[1] as usize;
    let mut packet_identifier = [0u8; 2];
    packet_identifier[0] = buffer_packet[topic_size + 2];
    packet_identifier[1] = buffer_packet[topic_size + 3];
    buffer_packet.remove(topic_size + 3);
    buffer_packet.remove(topic_size + 2);
    buffer_packet.insert(0, byte_0);
    let packet_to_server = PacketThings {
        thread_id: client.id,
        packet_type: Packet::Publish,
        bytes: buffer_packet,
    };
    let sender = client.sender.lock();
    match sender {
        Ok(sender_ok) => match sender_ok.send(packet_to_server) {
            Ok(_) => Ok(packet_identifier),
            Err(_) => Err("error".to_owned()),
        },
        Err(_) => Err("error".to_owned()),
    }
}

fn send_pingresp(client: &mut ClientFlags) {
    let buffer = [Packet::PingResp.into(), 0];

    client.connection.write_all(&buffer).unwrap();
    println!("Envié el PingResp");
    info!("Enviado PingResp");
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread;

    #[test]
    fn test01_nombre_protocolo_correto() {
        let nombre: [u8; 6] = [0x00, 0x04, 0x4D, 0x51, 0x54, 0x54];
        assert_eq!(verify_protocol_name(&nombre.to_vec()), Ok(()));
    }

    #[test]
    fn test02_nombre_protocolo_incorrecto() {
        let nombre: [u8; 6] = [0x00, 0x04, 0x4E, 0x51, 0x54, 0x14];
        assert_eq!(
            verify_protocol_name(&nombre.to_vec()),
            Err(INCORRECT_SERVER_CONNECTION)
        );
    }

    #[test]
    fn test03_version_protocolo_incorrecta() {
        let version: u8 = 45;
        assert_eq!(
            verify_version_protocol(&version),
            Err(CONNECTION_PROTOCOL_REJECTED)
        );
    }

    #[test]
    fn test04_get_tipo_connect_correcto() {
        let mut rng = rand::thread_rng();
        let header: u8 = 16 + rng.gen_range(0..16);
        let tipo = header.into();
        assert!(matches!(tipo, Packet::Connect));
    }

    #[test]
    fn test05_get_tipo_distinto_a_connect() {
        let header: u8 = 32;
        let tipo = header.into();
        assert_ne!(matches!(tipo, Packet::Connect), true);
    }

    #[test]
    fn test06_publish_pasado_al_coordinador() {
        let (clients_sender, coordinator_receiver): (Sender<PacketThings>, Receiver<PacketThings>) =
            mpsc::channel();
        let mutex_clients_sender = Arc::new(Mutex::new(clients_sender));
        let client_sender = Arc::clone(&mutex_clients_sender);
        let listener = TcpListener::bind("127.0.0.1:25525").unwrap();
        thread::spawn(move || loop {
            let _connection = listener.accept().unwrap();
        });
        let mut client = ClientFlags {
            id: 1,
            client_id: None,
            connection: &mut TcpStream::connect("127.0.0.1:25525").unwrap(),
            sender: client_sender,
            username: None,
            password: None,
            will_topic: None,
            will_message: None,
            will_qos: 0,
            will_retained: false,
            keep_alive: 1000,
        };
        let mut buffer_packet: Vec<u8> = Vec::new();
        let topic_subscribed = "as/tor".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_packet.push(0);
        buffer_packet.push(topic_subscribed_bytes.len() as u8);
        buffer_packet.append(&mut topic_subscribed_bytes);
        buffer_packet.push(3);
        buffer_packet.push(4);
        buffer_packet.push(5);
        buffer_packet.push(6);
        buffer_packet.push(7);
        buffer_packet.push(8);
        let byte_0: u8 = 0x31;
        make_publication(&mut client, buffer_packet, byte_0).unwrap();
        let packet_read = coordinator_receiver.recv().unwrap();
        assert_eq!(packet_read.thread_id, 1);
        let buff_read = packet_read.bytes;
        assert_eq!(buff_read[0], 0x31);
        assert_eq!(buff_read[2], 6);
        assert_eq!(buff_read[9], 5);
        assert_eq!(buff_read[10], 6);
        assert_eq!(buff_read[12], 8);
    }
}
