use crate::client::Client;
use crate::packet::Packet;
use crate::server::{bytes2string, PacketThings};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

pub fn run_coordinator(
    coordinator_receiver: Receiver<PacketThings>,
    lock_clients: Arc<Mutex<Vec<Client>>>,
) {
    loop {
        info!("Se lanzado el thread-coordinator");
        match coordinator_receiver.recv() {
            Ok(mut packet) => {
                match packet.packet_type {
                    Packet::Subscribe => {
                        info!("Se recibio un paquete Subscribe.");
                        let vector_with_qos = process_subscribe(&lock_clients, &packet);
                        send_subback(&lock_clients, packet, vector_with_qos)
                    }
                    Packet::Unsubscribe => {
                        info!("Se recibio un paquete Unsubscribe.");
                        unsubscribe_process(&lock_clients, &packet);
                        send_unsubback(&lock_clients, packet)
                    }
                    Packet::Publish => {
                        let topic_name = process_publish(&mut packet);
                        if topic_name.is_empty() {
                            continue;
                        }
                        send_publish_to_customer(&lock_clients, &mut packet, &topic_name)
                    }
                    _ => {
                        debug!("Se recibio un paquete desconocido.")
                    }
                }
                if lock_clients.lock().unwrap().is_empty() {
                    println!("Esta vacio, no lo ves?.");
                    debug!("No hay clientes.");
                }
            }
            Err(_e) => {}
        }
    }
}

fn send_publish_to_customer(
    lock_clients: &Arc<Mutex<Vec<Client>>>,
    packet: &mut PacketThings,
    topic_name: &str,
) {
    packet.bytes.remove(0);
    match lock_clients.lock() {
        Ok(locked) => {
            for client in locked.iter() {
                if client.is_subscribed_to(topic_name) {
                    let mut buffer_packet: Vec<u8> =
                        vec![Packet::Publish.into(), packet.bytes.len() as u8];
                    buffer_packet.append(&mut packet.bytes);
                    match client.channel.send(buffer_packet) {
                        Ok(_) => {
                            println!("Publish enviado al cliente")
                        }
                        Err(_) => {
                            println!("Error al enviar Publish al cliente")
                        }
                    }
                }
            }
        }
        Err(_) => {
            println!("Imposible acceder al lock desde el cordinador")
        }
    }
}

fn process_publish(packet: &mut PacketThings) -> String {
    let _byte_0 = packet.bytes[0]; //TODO Retained & qos
    let size = packet.bytes.len();
    let topic_name_len: usize = ((packet.bytes[1] as usize) << 8) + packet.bytes[2] as usize;
    let mut topic_name = String::from("");
    match bytes2string(&packet.bytes[3..(3 + topic_name_len)]) {
        Ok(value) => {
            topic_name = value;
        }
        Err(_) => {
            println!("Error procesando topico");
        }
    }
    let mut _topic_desc = String::from(""); //INICIO RETAINED
    if size > 3 + topic_name_len {
        match bytes2string(&packet.bytes[(5 + topic_name_len)..(size)]) {
            Ok(value) => {
                _topic_desc = value;
            }
            Err(_) => {
                println!("Error leyendo contenido del publish")
            }
        }
    }
    //TODO: AGREGAR RETAINED ACA
    topic_name
}

fn send_unsubback(lock_clients: &Arc<Mutex<Vec<Client>>>, packete: PacketThings) {
    let buffer: Vec<u8> = vec![
        Packet::UnsubAck.into(),
        0x02,
        packete.bytes[0],
        packete.bytes[1],
    ];
    match lock_clients.lock() {
        Ok(locked) => {
            if let Some(index) = locked.iter().position(|r| r.id == packete.thread_id) {
                match locked[index].channel.send(buffer) {
                    Ok(_) => {
                        println!("SubBack enviado.");
                        info!("SubBack enviado.");
                    }
                    Err(_) => {
                        println!("Error al enviar Unsubback.");
                        debug!("Error con el envio del Unsubback.")
                    }
                }
            }
        }
        Err(_) => {
            println!("Imposible acceder al lock desde el cordinador.");
            warn!("Imposible acceder al lock desde el cordinador.")
        }
    }
}

fn unsubscribe_process(lock_clients: &Arc<Mutex<Vec<Client>>>, packet: &PacketThings) {
    let mut index = 2;
    while index < packet.bytes.len() {
        let topic_size: usize =
            ((packet.bytes[index] as usize) << 8) + packet.bytes[index + 1] as usize;
        index += 2;
        let topico = bytes2string(&packet.bytes[index..(index + topic_size)]).unwrap(); //TODO Cambiar el unwrap
        index += topic_size;
        match lock_clients.lock() {
            Ok(mut locked) => {
                if let Some(indice) = locked.iter().position(|r| r.id == packet.thread_id) {
                    locked[indice].unsubscribe(topico);
                    info!("El cliente se desuscribio del topico.")
                }
            }
            Err(_) => {
                println!("Error al intentar desuscribir de un topico.");
                warn!("Error al intentar desuscribir de un topico.")
            }
        }
    }
}

fn send_subback(
    lock_clientes: &Arc<Mutex<Vec<Client>>>,
    packet: PacketThings,
    vector_with_qos: Vec<u8>,
) {
    let mut buffer: Vec<u8> = vec![
        Packet::SubAck.into(),
        (vector_with_qos.len() as u8 + 2_u8) as u8,
        packet.bytes[0],
        packet.bytes[1],
    ];
    for bytes in vector_with_qos {
        buffer.push(bytes);
    }
    match lock_clientes.lock() {
        Ok(locked) => {
            if let Some(index) = locked.iter().position(|r| r.id == packet.thread_id) {
                match locked[index].channel.send(buffer) {
                    Ok(_) => {
                        println!("SubBack enviado");
                        info!("SubBack enviado")
                    }
                    Err(_) => {
                        println!("Error al enviar Subback");
                        debug!("Error al enviar Subback")
                    }
                }
            }
        }
        Err(_) => {
            println!("Imposible acceder al lock desde el cordinador");
            warn!("Imposible acceder al lock desde el cordinador")
        }
    }
}

fn process_subscribe(lock_clients: &Arc<Mutex<Vec<Client>>>, packet: &PacketThings) -> Vec<u8> {
    let mut index = 2;
    let mut vector_with_qos: Vec<u8> = Vec::new();
    while index < (packet.bytes.len() - 2) {
        let topic_size: usize =
            ((packet.bytes[index] as usize) << 8) + packet.bytes[index + 1] as usize;
        index += 2;
        let topic = bytes2string(&packet.bytes[index..(index + topic_size)]).unwrap();
        index += topic_size;
        let qos: u8 = &packet.bytes[index] & 0x01;
        index += 1;
        match lock_clients.lock() {
            Ok(mut locked) => {
                if let Some(indice) = locked.iter().position(|r| r.id == packet.thread_id) {
                    locked[indice].subscribe(topic);
                    //TODO: ENVIAR RETAINED SI CORRESPONDE
                    vector_with_qos.push(qos);
                    info!("El cliente se subscribio al topico")
                }
            }
            Err(_) => vector_with_qos.push(0x80),
        }
    }
    vector_with_qos
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::sync::mpsc::Sender;
    use std::thread;

    #[test]
    fn test01_se_realiza_suscripcion_se_publica_el_coordinador_envia_ese_paquete() {
        let clients: Vec<Client> = Vec::new();
        let lock_clients = Arc::new(Mutex::new(clients));
        let handler_clients_locks = lock_clients.clone();
        let (clients_sender, coordinator_receiver): (Sender<PacketThings>, Receiver<PacketThings>) =
            mpsc::channel();
        let (coordinator_sender, client_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
            mpsc::channel();
        let mutex_clients_sender = Arc::new(Mutex::new(clients_sender));
        let client_sender = Arc::clone(&mutex_clients_sender);
        let client: Client = Client::new(1, coordinator_sender);
        handler_clients_locks.lock().unwrap().push(client);
        thread::Builder::new()
            .name("Coordinator".into())
            .spawn(move || run_coordinator(coordinator_receiver, lock_clients))
            .unwrap();
        let mut buffer_packet: Vec<u8> = Vec::new();
        let topic_subscribed = "as".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_packet.push(0);
        buffer_packet.push(54);
        buffer_packet.push(0);
        buffer_packet.push(topic_subscribed_bytes.len() as u8);
        buffer_packet.append(&mut topic_subscribed_bytes);
        buffer_packet.push(1);
        let packet_to_server = PacketThings {
            thread_id: 1,
            packet_type: Packet::Subscribe,
            bytes: buffer_packet,
        };
        client_sender
            .lock()
            .unwrap()
            .send(packet_to_server)
            .unwrap();
        let read_back = client_receiver.recv().unwrap();

        assert_eq!(read_back[0], 0x90);
        assert_eq!(read_back[3], 54);

        buffer_packet = Vec::new();
        let topic_publish = "as".to_owned();
        let mut topic_publish_bytes: Vec<u8> = topic_publish.as_bytes().to_vec();
        let content_publish = "miau".to_owned();
        let mut content_publish_bytes: Vec<u8> = content_publish.as_bytes().to_vec();
        buffer_packet.push(0x31);
        buffer_packet.push(0);
        buffer_packet.push(topic_publish_bytes.len() as u8);
        buffer_packet.append(&mut topic_publish_bytes);
        buffer_packet.append(&mut content_publish_bytes);
        let packet_to_server = PacketThings {
            thread_id: 1,
            packet_type: Packet::Publish,
            bytes: buffer_packet,
        };
        client_sender
            .lock()
            .unwrap()
            .send(packet_to_server)
            .unwrap();
        let read_back = client_receiver.recv().unwrap();
        let _packet_received_id = (read_back[0] >> 4) as u8;
        assert_eq!(read_back[0], 0x30);
        assert_eq!(read_back[4], 97);
        assert_eq!(read_back[5], 115);
        println!("{:?}", read_back);
    }
}
