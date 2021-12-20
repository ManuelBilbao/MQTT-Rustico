use crate::client::{Client, Subscription};
use crate::packet::{bytes2string, Packet, SUCCESSFUL_CONNECTION};
use crate::server::PacketThings;
use crate::utils::remaining_length_encode;
use crate::wildcard::compare_topic;
use rand::Rng;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

/// Receives messages from the Client Listener and take decisions
///
pub fn run_coordinator(
    coordinator_receiver: Receiver<PacketThings>,
    lock_clients: Arc<Mutex<HashMap<usize, Client>>>,
) {
    let mut retained_messages: HashMap<String, Vec<u8>> = HashMap::new();
    info!("Launched thread Coordinator.");
    loop {
        match coordinator_receiver.recv() {
            Ok(mut packet) => match packet.packet_type {
                Packet::Connect => {
                    info!("Connection packet received.");
                    process_client_id_and_info(&lock_clients, &mut packet, &retained_messages)
                }
                Packet::Subscribe => {
                    info!("Subscribe packet received.");
                    let vector_with_qos = process_subscribe(&lock_clients, &packet);
                    send_subback(&lock_clients, &packet, vector_with_qos);
                    send_retained_messages(&lock_clients, &packet, &mut retained_messages)
                }
                Packet::Unsubscribe => {
                    info!("Unsubscribe packet received.");
                    unsubscribe_process(&lock_clients, &packet);
                    send_unsubback(&lock_clients, packet)
                }
                Packet::Publish => {
                    debug!("Publish packet received.");
                    let topic_name = process_publish(&mut packet);
                    if topic_name.is_empty() {
                        continue;
                    }
                    if is_to_retained(&packet) {
                        let publish_packet =
                            send_publish_to_customer(&lock_clients, &mut packet, &topic_name);
                        retained_messages.insert(topic_name, publish_packet);
                    } else {
                        send_publish_to_customer(&lock_clients, &mut packet, &topic_name);
                    }
                }
                Packet::PubAck => {
                    remove_publishes(&lock_clients, &mut packet);
                }
                Packet::Disconnect => {
                    debug!("Disconnect packet received.");
                    close_process(&lock_clients, &packet);
                }
                Packet::Disgrace => {
                    debug!("Disgraceful disconnect packet received.");
                    close_disgraceful(&lock_clients, &packet);
                    let (publish_packet, topic_name, publish_message) =
                        send_lastwill(&lock_clients, &packet);
                    if !publish_packet.is_empty() {
                        retained_messages.insert(topic_name, publish_message.as_bytes().to_vec());
                    }
                }
                _ => {
                    debug!("Unknown packet received.")
                }
            },
            Err(_e) => {}
        }
    }
}

///
///
fn send_lastwill(
    lock_clients: &Arc<Mutex<HashMap<usize, Client>>>,
    packet: &PacketThings,
) -> (Vec<u8>, String, String) {
    let mut buffer_packet: Vec<u8> = Vec::new();
    let mut topic_name = "".to_owned();
    let mut topic_message = "".to_owned();
    match lock_clients.lock() {
        Ok(mut locked) => {
            let mut lastwill_exists = false;
            let mut lastwill_qos = 0;
            match locked.get_mut(&packet.thread_id) {
                Some(client) => {
                    if let Some(topic) = &client.lastwill_topic {
                        if let Some(message) = &client.lastwill_message {
                            lastwill_qos = client.lastwill_qos;
                            buffer_packet.push(Packet::Publish.into());
                            if lastwill_qos == 1 {
                                buffer_packet[0] |= 0x02;
                            }
                            topic_name = topic.clone();
                            topic_message = message.clone();
                            let mut topic_name_bytes = topic.as_bytes().to_vec();
                            let mut topic_message_bytes = message.as_bytes().to_vec();
                            buffer_packet.push(
                                (4 + topic_name_bytes.len() + topic_message_bytes.len()) as u8,
                            );
                            buffer_packet.push(0);
                            buffer_packet.push(topic_name_bytes.len() as u8);
                            buffer_packet.append(&mut topic_name_bytes);
                            let mut rng = rand::thread_rng();
                            let packet_id: u16 = rng.gen();
                            let packet_id_left: u8 = (packet_id >> 8) as u8;
                            let packet_id_right: u8 = packet_id as u8;
                            buffer_packet.push(packet_id_left);
                            buffer_packet.push(packet_id_right);
                            buffer_packet.append(&mut topic_message_bytes);
                            lastwill_exists = true;
                        }
                    }
                }
                None => {
                    warn!("Error searching client for send lastwill")
                }
            }
            if lastwill_exists {
                for (_, client_it) in locked.iter_mut() {
                    if client_it.is_subscribed_to(&topic_name) {
                        let mut buffer_to_send: Vec<u8> = Vec::new();
                        buffer_to_send.extend(&buffer_packet);
                        let mut buffer_clone = buffer_to_send.clone();
                        buffer_clone[0] |= 0x08;
                        if (!client_it.disconnected || client_it.clean_session == 0)
                            && lastwill_qos == 1
                            && client_it.is_subscribed_to_qos1(&topic_name)
                        {
                            client_it.publishes_received.push(buffer_clone);
                        }
                        if !client_it.disconnected {
                            match client_it.channel.send(buffer_to_send) {
                                Ok(_) => {
                                    debug!("Publish sent to customer.");
                                }
                                Err(_) => {
                                    debug!("Error sending publish to customer.")
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {
            warn!("Unable to get the clients lock.")
        }
    }
    (buffer_packet, topic_name, topic_message)
}
/// Set client as disconnect and remove subscripciones if need it.
///
fn close_process(lock_clients: &Arc<Mutex<HashMap<usize, Client>>>, packet: &PacketThings) {
    match lock_clients.lock() {
        Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
            Some(client) => {
                let close: Vec<u8> = vec![255_u8];
                match client.channel.send(close) {
                    Ok(_) => {}
                    Err(_) => {
                        warn!("Error sending secret packet.")
                    }
                }
                if client.clean_session == 1 {
                    client.remove_subscriptions_and_queue();
                }
                client.disconnected = true;
            }
            None => {
                debug!("Error looking client to erase.")
            }
        },
        Err(_) => {
            warn!("Unable to get the clients lock.")
        }
    }
}
/// Set client as disconnect and remove subscripciones if need it.
///
fn close_disgraceful(lock_clients: &Arc<Mutex<HashMap<usize, Client>>>, packet: &PacketThings) {
    match lock_clients.lock() {
        Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
            Some(client) => {
                if !client.disconnected {
                    let close: Vec<u8> = vec![255_u8];
                    match client.channel.send(close) {
                        Ok(_) => {}
                        Err(_) => {
                            warn!("Error sending the message")
                        }
                    }
                    if client.clean_session == 1 {
                        client.remove_subscriptions_and_queue();
                    }
                    client.disconnected = true;
                }
            }
            None => {
                debug!("Error trying to find usen on hash")
            }
        },
        Err(_) => {
            warn!("Unable to access lock from coordinador.")
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
                    info!("Publish removed from vector");
                    client.publishes_received.remove(indice2);
                }
            }
            None => {
                debug!("Client not found on hashmap")
            }
        },
        Err(_) => {
            warn!("Error trying to delete a publish.")
        }
    }
}

fn is_to_retained(packet: &PacketThings) -> bool {
    (packet.bytes[0] & 0x01) == 1
}

///
///
fn process_client_id_and_info(
    lock_clients: &Arc<Mutex<HashMap<usize, Client>>>,
    packet: &mut PacketThings,
    retained_msg: &HashMap<String, Vec<u8>>,
) {
    match lock_clients.lock() {
        Ok(mut locked) => {
            let mut already_exists = false;
            let size = packet.bytes[0] as usize;
            let new_client_id = bytes2string(&packet.bytes[1..(size + 1) as usize]);
            let clean_session = packet.bytes[(size + 1) as usize];
            let mut will_topic = None;
            let mut will_message = None;
            let mut will_qos = 0;
            let mut will_retained = false;
            if packet.bytes[(size + 2) as usize] == 1 {
                let mut index = (size + 3) as usize;
                let topic_size = packet.bytes[index] as usize;
                will_topic = Some(bytes2string(
                    &packet.bytes[(index + 1) as usize..(index + 1 + topic_size) as usize],
                ));
                index = (index + 1 + topic_size) as usize;
                let message_size = packet.bytes[(index) as usize] as usize;
                will_message = Some(bytes2string(
                    &packet.bytes[(index + 1) as usize..(index + 1 + message_size) as usize],
                ));
                will_qos = packet.bytes[(index + 1 + message_size) as usize];
                if packet.bytes[(index + 2 + message_size) as usize] == 1 {
                    will_retained = true;
                }
            }
            let mut subscriptions: Vec<Subscription> = Vec::new();
            let mut publishes_received: Vec<Vec<u8>> = Vec::new();
            let mut old_thread_id = 0;
            for client in locked.iter_mut() {
                if client.1.client_id == new_client_id {
                    already_exists = true;
                    subscriptions.append(&mut client.1.topics);
                    publishes_received.append(&mut client.1.publishes_received);
                    old_thread_id = client.1.thread_id;
                }
            }
            if already_exists {
                locked.remove(&old_thread_id);
            }

            match locked.get_mut(&packet.thread_id) {
                Some(client) => {
                    client.client_id = new_client_id;
                    if already_exists {
                        client.publishes_received.append(&mut publishes_received);
                        client.topics.append(&mut subscriptions);
                        for topic in client.topics.iter() {
                            for (topic_retained, message_retained) in retained_msg.iter() {
                                if compare_topic(topic_retained, &(topic.topic)) {
                                    client.publishes_received.push(message_retained.clone());
                                }
                            }
                        }
                    }
                    client.lastwill_topic = will_topic;
                    client.lastwill_message = will_message;
                    client.lastwill_qos = will_qos;
                    client.lastwill_retained = will_retained;
                    client.disconnected = false;
                    client.clean_session = clean_session;
                    if already_exists {
                        send_connection_result(client, SUCCESSFUL_CONNECTION, 1);
                    } else {
                        send_connection_result(client, SUCCESSFUL_CONNECTION, 0);
                    }
                }
                None => {
                    debug!("Client not found on hashmap")
                }
            }
        }
        Err(_) => {
            warn!("Unable to access lock from coordinador.")
        }
    }
}

fn send_connection_result(client: &mut Client, result_code: u8, session: u8) {
    let buffer = vec![Packet::ConnAck.into(), 0x02, session, result_code];

    match client.channel.send(buffer) {
        Ok(_) => {
            info!("Sent the connack sucessfull to the client sender");
        }
        Err(_) => {
            error!("Couldn't send connack to the client sender");
        }
    };
}

/// Sends the publish content to Client Communicator
///
fn send_publish_to_customer(
    lock_clients: &Arc<Mutex<HashMap<usize, Client>>>,
    packet: &mut PacketThings,
    topic_name: &str,
) -> Vec<u8> {
    let publish_qos = packet.bytes[0] & 0x02;
    packet.bytes.remove(0);
    let byte_publish: u8 = Packet::Publish.into();
    let byte_0: u8 = byte_publish | publish_qos;
    let mut buffer_packet: Vec<u8> = vec![byte_0];
    let mut remaining_length = remaining_length_encode(packet.bytes.len());
    buffer_packet.append(&mut remaining_length);
    buffer_packet.extend(&packet.bytes);
    match lock_clients.lock() {
        Ok(mut locked) => {
            for client in locked.iter_mut() {
                if client.1.is_subscribed_to(topic_name) {
                    let mut buffer_to_send: Vec<u8> = Vec::new();
                    buffer_to_send.extend(&buffer_packet);
                    let mut buffer_clone = buffer_to_send.clone();
                    buffer_clone[0] |= 0x08;
                    if (!client.1.disconnected || client.1.clean_session == 0)
                        && publish_qos == 2
                        && client.1.is_subscribed_to_qos1(topic_name)
                    {
                        client.1.publishes_received.push(buffer_clone);
                    }
                    if !client.1.disconnected {
                        match client.1.channel.send(buffer_to_send) {
                            Ok(_) => {
                                info!("Publish sent to cliente");
                            }
                            Err(_) => {
                                debug!("Error sending Publish to the client")
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {
            warn!("Unable to access lock from coordinador.")
        }
    }
    buffer_packet
}

fn process_publish(packet: &mut PacketThings) -> String {
    let topic_name_len: usize = ((packet.bytes[1] as usize) << 8) + packet.bytes[2] as usize;
    bytes2string(&packet.bytes[3..(3 + topic_name_len)])
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
                    info!("SubBack sent.");
                }
                Err(_) => {
                    debug!("Error sending Unsubback.")
                }
            },
            None => {
                warn!("Client not found on hashmap")
            }
        },
        Err(_) => {
            warn!("Unable to access lock from coordinador.")
        }
    }
}

fn unsubscribe_process(lock_clients: &Arc<Mutex<HashMap<usize, Client>>>, packet: &PacketThings) {
    let mut index = 2;
    while index < packet.bytes.len() {
        let topic_size: usize =
            ((packet.bytes[index] as usize) << 8) + packet.bytes[index + 1] as usize;
        index += 2;
        let topic = bytes2string(&packet.bytes[index..(index + topic_size)]);
        index += topic_size;
        match lock_clients.lock() {
            Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
                Some(client) => {
                    client.unsubscribe(topic);
                    info!("Cliente unsubscribed from a topic.")
                }
                None => {
                    debug!("Client not found on hashmap")
                }
            },
            Err(_) => {
                warn!("Error trying to unsubscribe from a topic.")
            }
        }
    }
}

fn send_subback(
    lock_clientes: &Arc<Mutex<HashMap<usize, Client>>>,
    packet: &PacketThings,
    vector_with_qos: Vec<u8>,
) {
    let mut buffer: Vec<u8> = vec![Packet::SubAck.into()];
    let mut remaining_length = remaining_length_encode(vector_with_qos.len() + 2);
    buffer.append(&mut remaining_length);
    buffer.push(packet.bytes[0]);
    buffer.push(packet.bytes[1]);
    for bytes in vector_with_qos {
        buffer.push(bytes);
    }
    match lock_clientes.lock() {
        Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
            Some(client) => match client.channel.send(buffer) {
                Ok(_) => {
                    info!("SubBack sent")
                }
                Err(_) => {
                    debug!("Error sending Subback")
                }
            },
            None => {
                warn!("Client not found on hashmap")
            }
        },
        Err(_) => {
            warn!("Unable to access lock from coordinador.")
        }
    }
}

fn process_subscribe(
    lock_clients: &Arc<Mutex<HashMap<usize, Client>>>,
    packet: &PacketThings,
) -> Vec<u8> {
    let mut index = 2;
    let mut vector_with_qos: Vec<u8> = Vec::new();
    while index < (packet.bytes.len() - 2) {
        let topic_size: usize =
            ((packet.bytes[index] as usize) << 8) + packet.bytes[index + 1] as usize;
        index += 2;
        let topic = bytes2string(&packet.bytes[index..(index + topic_size)]);
        index += topic_size;
        let qos: u8 = &packet.bytes[index] & 0x01;
        index += 1;
        match lock_clients.lock() {
            Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
                Some(client) => {
                    client.subscribe(topic.clone(), qos);
                    vector_with_qos.push(qos);
                    info!("Client subscribed to a topic")
                }
                None => {
                    error!("Client not found on hashmap")
                }
            },
            Err(_) => vector_with_qos.push(0x80),
        }
    }
    vector_with_qos
}

fn send_retained_messages(
    lock_clients: &Arc<Mutex<HashMap<usize, Client>>>,
    packet: &PacketThings,
    retained_messages: &mut HashMap<String, Vec<u8>>,
) {
    let mut index = 2;
    while index < (packet.bytes.len() - 2) {
        let topic_size: usize =
            ((packet.bytes[index] as usize) << 8) + packet.bytes[index + 1] as usize;
        index += 2;
        let topic = bytes2string(&packet.bytes[index..(index + topic_size)]);
        index += topic_size + 1;
        match lock_clients.lock() {
            Ok(mut locked) => match locked.get_mut(&packet.thread_id) {
                Some(client) => {
                    for (topic_retained, message_retained) in retained_messages.iter() {
                        if compare_topic(topic_retained, &topic) {
                            let mut buffer_to_send: Vec<u8> = Vec::new();
                            buffer_to_send.extend(message_retained);
                            match client.channel.send(buffer_to_send) {
                                Ok(_) => {
                                    info!("Publish retained sent to client")
                                }
                                Err(_) => {
                                    error!("Error sendint retained Publish to client")
                                }
                            }
                        }
                    }
                }
                None => {
                    error!("Client not found on hashmap")
                }
            },
            Err(_) => {
                error!("Unable to access lock from coordinador.")
            }
        }
    }
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
        //Arrange
        let clients: HashMap<usize, Client> = HashMap::new();
        let lock_clients = Arc::new(Mutex::new(clients));
        let handler_clients_locks = lock_clients.clone();
        let (clients_sender, coordinator_receiver): (Sender<PacketThings>, Receiver<PacketThings>) =
            mpsc::channel();
        let (coordinator_sender, client_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
            mpsc::channel();
        let mutex_clients_sender = Arc::new(Mutex::new(clients_sender));
        let client_sender = Arc::clone(&mutex_clients_sender);
        let mut client: Client = Client::new(1, coordinator_sender);
        client.disconnected = false;
        handler_clients_locks
            .lock()
            .unwrap()
            .insert(client.thread_id, client);
        thread::Builder::new()
            .name("Coordinator".into())
            .spawn(move || run_coordinator(coordinator_receiver, lock_clients))
            .unwrap();
        //Act Subscribe
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
        //Assert suback
        assert_eq!(read_back[0], 0x90);
        assert_eq!(read_back[3], 54);
        //Act publish
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
        //Assert publish to client
        assert_eq!(read_back[0], 0x30);
        assert_eq!(read_back[4], 97);
        assert_eq!(read_back[5], 115);
    }

    #[test]
    fn test02_se_le_envia_el_mismo_client_id_al_coordinador_y_elimina_el_cliente_auxiliar() {
        //Arrange
        let (channel_1, _c_1): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
        let (channel_2, _c_2): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
        let mut client_1 = Client {
            thread_id: 1,
            client_id: "Homero".to_owned(),
            channel: channel_1,
            topics: Vec::new(),
            publishes_received: Vec::new(),
            clean_session: 1,
            lastwill_topic: None,
            lastwill_message: None,
            lastwill_qos: 0,
            lastwill_retained: false,
            disconnected: false,
        };
        client_1.subscribe("as/tillero".to_owned(), 1);
        client_1.subscribe("ma/derero".to_owned(), 1);
        let client_2 = Client {
            thread_id: 2,
            client_id: "".to_owned(),
            channel: channel_2,
            topics: Vec::new(),
            publishes_received: Vec::new(),
            clean_session: 1,
            lastwill_topic: None,
            lastwill_message: None,
            lastwill_qos: 0,
            lastwill_retained: false,
            disconnected: false,
        };
        let mut clients: HashMap<usize, Client> = HashMap::new();
        clients.insert(client_1.thread_id, client_1);
        clients.insert(client_2.thread_id, client_2);
        let lock_clients = Arc::new(Mutex::new(clients));
        let handler_clients_locks = lock_clients.clone();
        let (clients_sender, coordinator_receiver): (Sender<PacketThings>, Receiver<PacketThings>) =
            mpsc::channel();
        thread::Builder::new()
            .name("Coordinator".into())
            .spawn(move || run_coordinator(coordinator_receiver, lock_clients))
            .unwrap();
        let mut bytes: Vec<u8> = Vec::new();
        let mut name = "Homero".to_owned().as_bytes().to_vec();
        bytes.push(name.len() as u8);
        bytes.append(&mut name);
        bytes.push(1); //clean_session
        bytes.push(0); //not lastwill
        let mut buffer_packet: Vec<u8> = Vec::new();
        buffer_packet.append(&mut bytes);
        let packet_to_server = PacketThings {
            thread_id: 2,
            packet_type: Packet::Connect,
            bytes: buffer_packet,
        };
        //Act connect
        clients_sender.send(packet_to_server).unwrap();
        //Assert
        thread::sleep(time::Duration::from_millis(20));
        let hashmap = handler_clients_locks.lock().unwrap();
        assert_eq!(hashmap.len(), 1);
        assert_eq!(hashmap.get(&2).unwrap().thread_id, 2);
        assert!(hashmap.get(&2).unwrap().is_subscribed_to("as/tillero"));
        assert!(!hashmap.get(&2).unwrap().is_subscribed_to("am/tillero"));
    }

    #[test]
    fn test03_se_realiza_suscripcion_y_desuscripcion() {
        //Arrange
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
        //Act Subscribe
        let mut buffer_packet: Vec<u8> = Vec::new();
        let topic_subscribed = "pepitoelpistolero".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_packet.push(0);
        buffer_packet.push(30); //PacketID
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
        //Assert suback
        assert_eq!(read_back[0], 0x90);
        assert_eq!(read_back[3], 30); //PacketID
                                      //Act unsuscribe
        buffer_packet = Vec::new();
        let topic_unsuscribe = "pepitoelpistolero".to_owned();
        let mut topic_unsuscribe_bytes: Vec<u8> = topic_unsuscribe.as_bytes().to_vec();
        buffer_packet.push(0);
        buffer_packet.push(32); //PacketID
        buffer_packet.push(0);
        buffer_packet.push(topic_unsuscribe_bytes.len() as u8);
        buffer_packet.append(&mut topic_unsuscribe_bytes);
        let packet_to_server = PacketThings {
            thread_id: 1,
            packet_type: Packet::Unsubscribe,
            bytes: buffer_packet,
        };
        client_sender
            .lock()
            .unwrap()
            .send(packet_to_server)
            .unwrap();
        let read_back = client_receiver.recv().unwrap();
        let _packet_received_id = (read_back[0] >> 4) as u8;
        //Assert unsuscribe
        assert_eq!(read_back[0], 0xB0);
        assert_eq!(read_back[1], 2);
        assert_eq!(read_back[2], 0);
        assert_eq!(read_back[3], 32);
    }
}
