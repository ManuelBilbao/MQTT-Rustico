use crate::client::Client;
use crate::packet::{bytes2string, Packet};
use crate::server::PacketThings;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

pub struct RetainedMessage {
    topic: String,
    message: Vec<u8>,
}

pub fn run_coordinator(
    coordinator_receiver: Receiver<PacketThings>,
    lock_clients: Arc<Mutex<HashMap<usize, Client>>>,
) {
    let mut retained_messages = Vec::new();
    loop {
        info!("Se lanzado el thread-coordinator");
        match coordinator_receiver.recv() {
            Ok(mut packet) => {
                match packet.packet_type {
                    Packet::Connect => process_client_id(&lock_clients, &mut packet),
                    Packet::Subscribe => {
                        info!("Se recibio un paquete Subscribe.");
                        let vector_with_qos =
                            process_subscribe(&lock_clients, &packet, &mut retained_messages);
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
                        if is_to_retained(&packet) {
                            let publish_packet =
                                send_publish_to_customer(&lock_clients, &mut packet, &topic_name);
                            let new_retained = RetainedMessage {
                                topic: topic_name,
                                message: publish_packet,
                            };
                            retained_messages.push(new_retained);
                        } else {
                            send_publish_to_customer(&lock_clients, &mut packet, &topic_name);
                        }
                    }
                    Packet::PubAck => {
                        remove_publishes(&lock_clients, &mut packet);
                    }
                    Packet::Disconnect => {
                        info!("Se recibio un paquete Disconnect.");
                        close_process(&lock_clients, &packet);
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

fn close_process(lock_clients: &Arc<Mutex<HashMap<usize, Client>>>, packet: &PacketThings) {
    match lock_clients.lock() {
        Ok(mut locked) => {
            locked.remove(&packet.thread_id);
        }
        Err(_) => {
            println!("Imposible acceder al lock desde el cordinador")
        }
    }
}

fn remove_publishes(lock_clients: &Arc<Mutex<HashMap<usize, Client>>>, packet: &mut PacketThings) {
    let puback_packet_identifier = ((packet.bytes[0] as u16) << 8) + packet.bytes[1] as u16;
    match lock_clients.lock() {
        Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
            Some(client) => {
                if let Some(indice2) = client.publishes_received.iter().position(|r| {
                    let topic_name_len: usize = ((r[2] as usize) << 8) + r[3] as usize;
                    let publish_packet_identifier =
                        ((r[topic_name_len + 4] as u16) << 8) + r[topic_name_len + 5] as u16;
                    publish_packet_identifier == puback_packet_identifier
                }) {
                    println!("eliminado publish del vector");
                    client.publishes_received.remove(indice2);
                }
            }
            None => {
                println!("Cliente no encontrado en hashmap")
            }
        },
        Err(_) => {
            println!("Error al intentar eliminar un publish.");
            warn!("Error al intentar eliminar un publish.")
        }
    }
}

fn is_to_retained(packet: &PacketThings) -> bool {
    (packet.bytes[0] & 0x01) == 1
}

fn process_client_id(lock_clients: &Arc<Mutex<HashMap<usize, Client>>>, packet: &mut PacketThings) {
    match lock_clients.lock() {
        Ok(mut locked) => {
            let mut already_exists = false;
            let new_client_id = bytes2string(&packet.bytes[0..(packet.bytes.len())]).unwrap();
            let mut subscriptions: Vec<String> = Vec::new();
            let mut old_thread_id = 0;
            for client in locked.iter_mut() {
                if client.1.client_id == new_client_id {
                    already_exists = true;
                    subscriptions.append(&mut client.1.topics);
                    old_thread_id = client.1.thread_id;
                }
            }
            locked.remove(&old_thread_id);

            match locked.get_mut(&packet.thread_id) {
                Some(client) => {
                    client.client_id = new_client_id;
                    if already_exists {
                        client.topics.append(&mut subscriptions);
                    }
                }
                None => {
                    println!("cliente no encontrado en hashmap")
                }
            }
        }
        Err(_) => {
            println!("Imposible acceder al lock desde el cordinador")
        }
    }
}

fn send_publish_to_customer(
    lock_clients: &Arc<Mutex<HashMap<usize, Client>>>,
    packet: &mut PacketThings,
    topic_name: &str,
) -> Vec<u8> {
    packet.bytes.remove(0);
    let mut buffer_packet: Vec<u8> = vec![Packet::Publish.into(), packet.bytes.len() as u8];
    buffer_packet.extend(&packet.bytes);
    match lock_clients.lock() {
        Ok(mut locked) => {
            for client in locked.iter_mut() {
                if client.1.is_subscribed_to(topic_name) {
                    let mut buffer_to_send: Vec<u8> = Vec::new();
                    buffer_to_send.extend(&buffer_packet);
                    let buffer_clone = buffer_to_send.clone();
                    match client.1.channel.send(buffer_to_send) {
                        Ok(_) => {
                            println!("Publish enviado al cliente");
                            client.1.publishes_received.push(buffer_clone);
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
    buffer_packet
}

fn process_publish(packet: &mut PacketThings) -> String {
    let _byte_0 = packet.bytes[0]; //TODO qos
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
    topic_name
}

fn send_unsubback(lock_clients: &Arc<Mutex<HashMap<usize, Client>>>, packet: PacketThings) {
    let buffer: Vec<u8> = vec![
        Packet::UnsubAck.into(),
        0x02,
        packet.bytes[0],
        packet.bytes[1],
    ];
    match lock_clients.lock() {
        Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
            Some(client) => match client.channel.send(buffer) {
                Ok(_) => {
                    println!("SubBack enviado.");
                    info!("SubBack enviado.");
                }
                Err(_) => {
                    println!("Error al enviar Unsubback.");
                    debug!("Error con el envio del Unsubback.")
                }
            },
            None => {
                println!("cliente no encontrado en hashmap")
            }
        },
        Err(_) => {
            println!("Imposible acceder al lock desde el cordinador.");
            warn!("Imposible acceder al lock desde el cordinador.")
        }
    }
}

fn unsubscribe_process(lock_clients: &Arc<Mutex<HashMap<usize, Client>>>, packet: &PacketThings) {
    let mut index = 2;
    while index < packet.bytes.len() {
        let topic_size: usize =
            ((packet.bytes[index] as usize) << 8) + packet.bytes[index + 1] as usize;
        index += 2;
        let topico = bytes2string(&packet.bytes[index..(index + topic_size)]).unwrap(); //TODO Cambiar el unwrap
        index += topic_size;
        match lock_clients.lock() {
            Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
                Some(client) => {
                    client.unsubscribe(topico);
                    info!("El cliente se desuscribio del topico.")
                }
                None => {
                    println!("cliente no encontrado en hashmap")
                }
            },
            Err(_) => {
                println!("Error al intentar desuscribir de un topico.");
                warn!("Error al intentar desuscribir de un topico.")
            }
        }
    }
}

fn send_subback(
    lock_clientes: &Arc<Mutex<HashMap<usize, Client>>>,
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
        Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
            Some(client) => match client.channel.send(buffer) {
                Ok(_) => {
                    println!("SubBack enviado");
                    info!("SubBack enviado")
                }
                Err(_) => {
                    println!("Error al enviar Subback");
                    debug!("Error al enviar Subback")
                }
            },
            None => {
                println!("cliente no encontrado en hashmap")
            }
        },
        Err(_) => {
            println!("Imposible acceder al lock desde el cordinador");
            warn!("Imposible acceder al lock desde el cordinador")
        }
    }
}

fn process_subscribe(
    lock_clients: &Arc<Mutex<HashMap<usize, Client>>>,
    packet: &PacketThings,
    retained_messages: &mut Vec<RetainedMessage>,
) -> Vec<u8> {
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
            Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
                Some(client) => {
                    client.subscribe(topic.clone());
                    if let Some(indice_retained) =
                        retained_messages.iter().position(|r| r.topic == topic)
                    {
                        let mut buffer_to_send: Vec<u8> = Vec::new();
                        buffer_to_send.extend(&retained_messages[indice_retained].message);
                        match client.channel.send(buffer_to_send) {
                            Ok(_) => {
                                println!("Publish enviado al cliente")
                            }
                            Err(_) => {
                                println!("Error al enviar Publish al cliente")
                            }
                        }
                    }
                    vector_with_qos.push(qos);
                    info!("El cliente se subscribio al topico")
                }
                None => {
                    println!("cliente no encontrado en hashmap")
                }
            },
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
    use std::time;

    #[test]
    fn test01_se_realiza_suscripcion_se_publica_el_coordinador_envia_ese_paquete() {
        let clients: HashMap<usize, Client> = HashMap::new();
        let lock_clients = Arc::new(Mutex::new(clients));
        let handler_clients_locks = lock_clients.clone();
        let (clients_sender, coordinator_receiver): (Sender<PacketThings>, Receiver<PacketThings>) =
            mpsc::channel();
        let (coordinator_sender, client_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
            mpsc::channel();
        let mutex_clients_sender = Arc::new(Mutex::new(clients_sender));
        let client_sender = Arc::clone(&mutex_clients_sender);
        let client: Client = Client::new(1, coordinator_sender);
        handler_clients_locks
            .lock()
            .unwrap()
            .insert(client.thread_id, client);
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

    #[test]
    fn test02_se_le_envia_el_mismo_client_id_al_coordinador_y_elimina_el_cliente_auxiliar() {
        let (channel_1, _c_1): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
        let (channel_2, _c_2): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
        let mut client_1 = Client {
            thread_id: 1,
            client_id: "Homero".to_owned(),
            channel: channel_1,
            topics: Vec::new(),
            publishes_received: Vec::new(),
        };
        client_1.subscribe("as/tillero".to_owned());
        client_1.subscribe("ma/derero".to_owned());
        let client_2 = Client {
            thread_id: 2,
            client_id: "".to_owned(),
            channel: channel_2,
            topics: Vec::new(),
            publishes_received: Vec::new(),
        };
        let clients: HashMap<usize, Client> = HashMap::from([
            (client_1.thread_id, client_1),
            (client_2.thread_id, client_2),
        ]);
        let lock_clients = Arc::new(Mutex::new(clients));
        let handler_clients_locks = lock_clients.clone();
        let (clients_sender, coordinator_receiver): (Sender<PacketThings>, Receiver<PacketThings>) =
            mpsc::channel();
        thread::Builder::new()
            .name("Coordinator".into())
            .spawn(move || run_coordinator(coordinator_receiver, lock_clients))
            .unwrap();
        let mut bytes: Vec<u8> = "Homero".to_owned().as_bytes().to_vec();
        let mut buffer_packet: Vec<u8> = Vec::new();
        buffer_packet.append(&mut bytes);
        let packet_to_server = PacketThings {
            thread_id: 2,
            packet_type: Packet::Connect,
            bytes: buffer_packet,
        };
        clients_sender.send(packet_to_server).unwrap();
        thread::sleep(time::Duration::from_millis(20));
        let hashmap = handler_clients_locks.lock().unwrap();
        assert_eq!(hashmap.len(), 1);
        assert_eq!(hashmap.get(&2).unwrap().thread_id, 2);
        assert!(hashmap.get(&2).unwrap().is_subscribed_to("as/tillero"));
        assert!(!hashmap.get(&2).unwrap().is_subscribed_to("am/tillero"));
    }
}
