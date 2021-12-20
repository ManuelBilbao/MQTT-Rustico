//! # Packet
//!
//! Different packet management and processing.
use crate::server::{ClientFlags, PacketThings};
use std::io::{Read, Write};
use std::net::Shutdown;
use std::time::Duration;
use tracing::{debug, error, info, warn};

const MQTT_VERSION: u8 = 4;
const MQTT_NAME: [u8; 6] = [0x00, 0x04, 0x4D, 0x51, 0x54, 0x54];
const CONNECTION_USER_OR_PASS_REFUSED: u8 = 4;
const CONNECTION_IDENTIFIER_REFUSED: u8 = 2;
const CONNECTION_PROTOCOL_REJECTED: u8 = 1;
const INCORRECT_SERVER_CONNECTION: u8 = 1;
pub const SUCCESSFUL_CONNECTION: u8 = 0;

/// Type of recognized packets.
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
    Disgrace,
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
            _ => Packet::Disgrace,
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
            _ => 0x00,
        }
    }
}

/// Read entire packet from stream and execute the function according to type.
///
/// # Errors
///
/// Returns Error if couldn't read from stream.
pub fn read_packet(
    client: &mut ClientFlags,
    packet_type: Packet,
    buffer_size: usize,
    byte_0: u8,
    password_required: bool,
) -> Result<(), std::io::Error> {
    let mut buffer_packet: Vec<u8> = vec![0; buffer_size];
    client.connection.read_exact(&mut buffer_packet)?;
    match packet_type {
        Packet::Connect => match make_connection(client, buffer_packet, password_required) {
            Ok(_) => {}
            Err(error_code) => {
                send_connection_error(client, error_code);
            }
        },
        Packet::Publish => match make_publication(client, buffer_packet, byte_0) {
            Ok(paquete_identifier) => {
                if (byte_0 & 0x02) == 2 {
                    send_publication_results(client, paquete_identifier);
                }
            }
            Err(_) => {
                warn!("Error when publishing.");
            }
        },
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
            inform_client_disconnect_to_coordinator(client, buffer_packet, Packet::Disconnect);
            close_streams(client);
        }
        Packet::PubAck => {
            remove_from_client_publishes(client, buffer_packet);
        }
        _ => {}
    }

    Ok(())
}

/// Inform coordinator that the client sent a _Disconnect_ packet.
pub fn inform_client_disconnect_to_coordinator(
    client: &mut ClientFlags,
    buffer_packet: Vec<u8>,
    tipo: Packet,
) {
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
                    info!("Success sending disconnect to coordinator thread.")
                }
                Err(_) => {
                    warn!("Error sending sending disconnect to coordinator thread.")
                }
            };
        }
        Err(_) => {
            warn!("Error reading coordinator channel.")
        }
    }
}

/// Close the stream with the client.
fn close_streams(client: &mut ClientFlags) {
    let mut buffer = Vec::new();
    let aux = client.connection.read_to_end(&mut buffer);

    match aux {
        Ok(0) => {
            info!("Client has closed the stream.")
        }
        _ => {
            client
                .connection
                .shutdown(Shutdown::Both)
                .expect("shutdown call failed");
            info!("Stream with client closed.");
        }
    }
    match client.sender.lock() {
        Ok(locked) => {
            drop(locked);
        }
        Err(_) => {
            error!("Error accessing the lock.");
        }
    }
    info!("Closed stream with Coordinator.");
}

/// Inform the coordinator that a new client has connected.
fn inform_new_connection(
    client: &mut ClientFlags,
    will_topic: Option<String>,
    will_message: Option<String>,
    will_qos: u8,
    will_retain: bool,
) {
    if let Some(client_id) = &client.client_id {
        let mut client_id_bytes = client_id.as_bytes().to_vec();
        let mut bytes: Vec<u8> = vec![client_id_bytes.len() as u8];
        bytes.append(&mut client_id_bytes);
        bytes.push(client.clean_session);
        match will_topic {
            Some(topic) => {
                bytes.push(1);
                if let Some(message) = will_message {
                    let mut topic_bytes = topic.as_bytes().to_vec();
                    let mut message_bytes = message.as_bytes().to_vec();
                    bytes.push(topic_bytes.len() as u8);
                    bytes.append(&mut topic_bytes);
                    bytes.push(message_bytes.len() as u8);
                    bytes.append(&mut message_bytes);
                    bytes.push(will_qos);
                    if will_retain {
                        bytes.push(1);
                    } else {
                        bytes.push(0);
                    }
                }
            }
            None => {
                bytes.push(0);
            }
        }
        let packet_to_server = PacketThings {
            thread_id: client.id,
            packet_type: Packet::Connect,
            bytes,
        };
        let sender = client.sender.lock();
        match sender {
            Ok(sender_ok) => {
                match sender_ok.send(packet_to_server) {
                    Ok(_) => {
                        info!("Success sending client id change to the Coordinator thread")
                    }
                    Err(_) => {
                        warn!("Error sending client id change to the Coordinator thread")
                    }
                };
            }
            Err(_) => {
                warn!("Error reading coordinator channel.")
            }
        }
    }
}

/// Inform coordinator that the client has changed a subscription.
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
                    info!("Success sending subscription change to the coordinator thread.")
                }
                Err(_) => {
                    debug!("Error sending subscription change to coordinator thread.")
                }
            };
        }
        Err(_) => {
            warn!("Error reading subscription change.")
        }
    }
}

/// Check if protocol name is the required for a new connection.
pub fn verify_protocol_name(buffer: &[u8]) -> Result<(), u8> {
    for i in 0..6 {
        if buffer[i] != MQTT_NAME[i] {
            debug!("Wrong connection protocol name");
            return Err(INCORRECT_SERVER_CONNECTION);
        }
    }
    Ok(())
}

/// Check if protocol version is the required for a new connection.
pub fn verify_version_protocol(level: &u8) -> Result<(), u8> {
    if *level == MQTT_VERSION {
        return Ok(());
    }
    Err(CONNECTION_PROTOCOL_REJECTED)
}

/// Send a _Connack_ packet to the client with the connection error code.
pub fn send_connection_error(client: &mut ClientFlags, result_code: u8) {
    let mut buffer = [0u8; 4];
    buffer[0] = Packet::ConnAck.into();
    buffer[1] = 0x02;
    buffer[2] = 0;
    buffer[3] = result_code;

    match client.connection.write_all(&buffer) {
        Ok(_) => {
            info!("Sent Connack packet with error code.");
        }
        Err(_) => {
            error!("Error communicating with the client.");
        }
    }
}

/// Send _Puback_ packet to the client.
fn send_publication_results(client: &mut ClientFlags, packet_identifier: [u8; 2]) {
    let mut buffer = [0u8; 4];
    buffer[0] = Packet::PubAck.into();
    buffer[1] = 0x02;
    buffer[2] = packet_identifier[0];
    buffer[3] = packet_identifier[1];

    match client.connection.write_all(&buffer) {
        Ok(_) => {
            info!("Puback packet sent");
        }
        Err(_) => {
            warn!("Error sending Puback packet");
        }
    }
}

/// Send new _Publish_ packet to the coordinator.
///
/// # Errors
///
/// Returns Error if:
/// - Couldn't lock de Sender Channel.
/// - Couldn't send the message to the coordinator.
fn make_publication(
    client: &mut ClientFlags,
    mut buffer_packet: Vec<u8>,
    byte_0: u8,
) -> Result<[u8; 2], String> {
    let topic_size: usize = ((buffer_packet[0] as usize) << 8) + buffer_packet[1] as usize;
    let mut packet_identifier = [0u8; 2];
    if(byte_0 & 0x02) == 2 {
        packet_identifier[0] = buffer_packet[topic_size + 2];
        packet_identifier[1] = buffer_packet[topic_size + 3];
    }
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
            Err(_) => Err("Error".to_owned()),
        },
        Err(_) => Err("Error".to_owned()),
    }
}

fn remove_from_client_publishes(client: &mut ClientFlags, buffer_packet: Vec<u8>) {
    let packet_to_server = PacketThings {
        thread_id: client.id,
        packet_type: Packet::PubAck,
        bytes: buffer_packet,
    };
    let sender = client.sender.lock();
    match sender {
        Ok(sender_ok) => match sender_ok.send(packet_to_server) {
            Ok(_) => {}
            Err(_) => {
                error!("Error sending message to client.")
            }
        },
        Err(_) => error!("Error sending message to client."),
    }
}

/// Send _Pingresp_ to the client.
fn send_pingresp(client: &mut ClientFlags) {
    let buffer = [Packet::PingResp.into(), 0];

    match client.connection.write_all(&buffer) {
        Ok(_) => {
            debug!("Pingresp packet sent.");
        }
        Err(_) => {
            error!("Error sending Pingresp.");
        }
    }
}

/// Process _Connection_ packet.
pub fn make_connection(
    client: &mut ClientFlags,
    buffer_packet: Vec<u8>,
    password_required: bool,
) -> Result<u8, u8> {
    verify_protocol_name(&buffer_packet)?;
    verify_version_protocol(&buffer_packet[6])?;

    let flag_username = buffer_packet[7] & 0x80 == 0x80;
    let flag_password = buffer_packet[7] & 0x40 == 0x40;
    let flag_will_retain = buffer_packet[7] & 0x20 == 0x20;
    let flag_will_qos = (buffer_packet[7] & 0x18) >> 3;
    let flag_will_flag = buffer_packet[7] & 0x04 == 0x04;
    let flag_clean_session = buffer_packet[7] & 0x02 == 0x02;

    let keep_alive: u16 = ((buffer_packet[8] as u16) << 8) + buffer_packet[9] as u16;

    let size_client_id: usize = ((buffer_packet[10] as usize) << 8) + buffer_packet[11] as usize;

    let mut client_id = None;

    if size_client_id != 0 {
        client_id = Some(bytes2string(&buffer_packet[12..12 + size_client_id]));
    }

    if client_id == None {
        return Err(CONNECTION_IDENTIFIER_REFUSED);
    }

    if password_required && (!flag_username || !flag_password) {
        return Err(CONNECTION_IDENTIFIER_REFUSED);
    }

    let mut index: usize = (12 + size_client_id) as usize;

    let mut will_topic = None;
    let mut will_message = None;
    if flag_will_flag {
        let size_will_topic: usize =
            ((buffer_packet[index] as usize) << 8) + buffer_packet[(index + 1)] as usize;
        index += 2_usize;
        will_topic = Some(bytes2string(
            &buffer_packet[index..(index + size_will_topic)],
        ));
        index += size_will_topic;

        let size_will_message: usize =
            ((buffer_packet[index] as usize) << 8) + buffer_packet[(index + 1) as usize] as usize;
        index += 2_usize;
        will_message = Some(bytes2string(
            &buffer_packet[index..(index + size_will_message)],
        ));
        index += size_will_message;
    }

    let mut username: Option<String> = None;
    if flag_username {
        let size_username: usize =
            ((buffer_packet[index] as usize) << 8) + buffer_packet[(index + 1) as usize] as usize;
        index += 2_usize;
        username = Some(bytes2string(&buffer_packet[index..(index + size_username)]));
        index += size_username;
    }

    let mut password: Option<String> = None;
    if flag_password {
        let size_password: usize =
            ((buffer_packet[index] as usize) << 8) + buffer_packet[(index + 1) as usize] as usize;
        index += 2_usize;
        password = Some(bytes2string(&buffer_packet[index..(index + size_password)]));
    }

    if let Some(user) = &username {
        if let Some(pass) = &password {
            if !user_and_password_correct(user, pass) {
                return Err(CONNECTION_USER_OR_PASS_REFUSED);
            }
        }
    }

    if keep_alive > 0 {
        let wait = (f32::from(keep_alive) * 1.5) as u64;
        if client
            .connection
            .set_read_timeout(Some(Duration::new(wait, 0)))
            .is_err()
        {
            error!("Error establishing the limit time for a client.")
        }
    }

    client.client_id = client_id;
    client.clean_session = flag_clean_session as u8;
    debug!("Clean session {}", flag_clean_session);
    client.keep_alive = keep_alive;

    inform_new_connection(
        client,
        will_topic,
        will_message,
        flag_will_qos,
        flag_will_retain,
    );
    Ok(1)
}

/// Convert bytes to UTF-8 string.
pub fn bytes2string(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(str) => str.to_owned(),
        Err(_) => "".to_owned(),
    }
}

/// Check if username and password are valid.
fn user_and_password_correct(user: &str, password: &str) -> bool {
    let file: String = match std::fs::read_to_string("./src/users.txt") {
        Ok(file) => file,
        Err(_) => return false,
    };
    let lines = file.lines();

    for line in lines {
        let name_and_pass: Vec<&str> = line.split('=').collect();
        let username: String = name_and_pass[0].to_string();
        let pass: String = name_and_pass[1].to_string();
        if username == user {
            if pass == password {
                return true;
            }
            return false;
        }
    }
    false
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
            clean_session: 1,
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
        assert_eq!(buff_read[9], 3);
        assert_eq!(buff_read[10], 4);
        assert_eq!(buff_read[12], 6);
        assert_eq!(buff_read[14], 8);
    }
}
