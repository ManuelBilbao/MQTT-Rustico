use crate::utils::remaining_length_encode;
use crate::{
    calculate_connection_length, create_byte_with_flags, FlagsConnection, UserInformation,
};
use rand::Rng;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::Sender;

const MQTT_VERSION: u8 = 4;

/// All possible packet types
///
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
    fn from(package: Packet) -> Self {
        match package {
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
/// Receives byte_0 which is used to get the packet type and then reads the rest of the packet. Then it calls a different function for each packet type.
///
pub fn read_packet(
    stream: &mut TcpStream,
    byte_0: u8,
    buffer_size: usize,
    puback_sender: Sender<String>,
    message_sender: Sender<String>,
    connack_sender: Sender<String>,
    topic_update_sender: Sender<String>,
) -> Result<(), std::io::Error> {
    let mut buffer_paquete: Vec<u8> = vec![0; buffer_size];
    let mut stream_clone = stream.try_clone().unwrap();
    let package_type: Packet = byte_0.into();
    stream.read_exact(&mut buffer_paquete)?;
    match package_type {
        Packet::ConnAck => read_connack(buffer_paquete, connack_sender),
        Packet::PubAck => read_puback(buffer_paquete, puback_sender),
        Packet::SubAck => read_suback(buffer_paquete),
        Packet::UnsubAck => read_unsuback(buffer_paquete),
        Packet::PingResp => read_pingresp(),
        Packet::Publish => read_publish(
            byte_0,
            buffer_paquete,
            &mut stream_clone,
            message_sender,
            topic_update_sender,
        ),
        _ => {
            // Manejar
            println!("Unknown packet received");
        }
    }
    Ok(())
}

/// Reads connack packet that contains session_present and return_code then informs the UI that the connection was successfull.
///
pub fn read_connack(buffer: Vec<u8>, connack_sender: Sender<String>) {
    let session_present = buffer[0];
    let return_code = buffer[1];
    println!(
        "Recibe connack con sp: {} y return code {}",
        session_present, return_code
    );
    connack_sender
        .send("Connected successfully\n".to_string())
        .expect("Error when sending text to UI");
}
/// Reads puback packet that contains packet_identifier then informs UI that the publish was successfull.
///
pub fn read_puback(buffer: Vec<u8>, puback_sender: Sender<String>) {
    let _packet_identifier = ((buffer[0] as u16) << 8) + buffer[1] as u16;
    puback_sender
        .send("Publish sent successfully\n".to_string())
        .expect("Error When sending text to UI");
}
/// Reads suback packet that contains packet_identifier and list of return codes that specify the maximum QoS level that was granted for each subscription.
///
pub fn read_suback(buffer: Vec<u8>) {
    let _packet_identifier = ((buffer[0] as u16) << 8) + buffer[1] as u16;
    let amount_of_topics = buffer.len() - 2;
    let mut topic_results: Vec<u8> = Vec::new();
    for i in 0..amount_of_topics {
        topic_results.push(buffer[i + 2]);
    }
}
/// Reads unsuback packet that contains packet_identifier.
///
pub fn read_unsuback(buffer: Vec<u8>) {
    let packet_identifier = ((buffer[0] as u16) << 8) + buffer[1] as u16;
    println!(
        "Recibido UnsubAck. Packet Identifier: {}.",
        packet_identifier
    );
}
/// Does nothing as pingresp only has a fixed header tha was already read.
///
pub fn read_pingresp() {
    // PingResp is empty
}
/// Reads publish packet that contains a topic and a message, and then sends these to the UI. The topic is also sent to the UI through topic_update_sender to
/// update the subscribed topics label, in case the client receives a message with a topic to which the client had subscribed to in a previous session.
/// If QoS is 1 then a puback packet is also sent to the server.
///
pub fn read_publish(
    byte_0: u8,
    buffer: Vec<u8>,
    stream: &mut TcpStream,
    message_sender: Sender<String>,
    topic_update_sender: Sender<String>,
) {
    let topic_name_len: usize = ((buffer[0] as usize) << 8) + buffer[1] as usize;
    match bytes2string(&buffer[2..(2 + topic_name_len)]) {
        Ok(mut topic_name) => {
            topic_update_sender
                .send(topic_name.clone())
                .expect("Error when sending text to UI");
            topic_name += " - ";
            let mut sum_index: usize = 2;
            if (byte_0 & 0x02) == 2 {
                sum_index = 4;
            }
            match bytes2string(&buffer[(sum_index + topic_name_len)..buffer.len()]) {
                Ok(mut message) => {
                    message += "\n";
                    let topic_and_message = topic_name + &message;
                    message_sender
                        .send(topic_and_message)
                        .expect("Error when sending text to UI");
                }
                Err(_) => {
                    println!("Error processing message");
                }
            }
        }
        Err(_) => {
            println!("Error processing topic");
        }
    }
    if (byte_0 & 0x02) == 2 {
        let mut packet_identifier = [0u8; 2];
        packet_identifier[0] = buffer[topic_name_len + 2];
        packet_identifier[1] = buffer[topic_name_len + 3];
        send_puback_packet(stream, packet_identifier);
    }
}

pub fn bytes2string(bytes: &[u8]) -> Result<String, u8> {
    match std::str::from_utf8(bytes) {
        Ok(str) => Ok(str.to_owned()),
        Err(_) => Err(1),
    }
}
/// Creates the connection packet according to mqtt protocol and then sends it to server
pub fn send_packet_connection(
    stream: &mut TcpStream,
    flags: FlagsConnection,
    user_information: UserInformation,
) {
    let buffer_size = calculate_connection_length(&flags, &user_information);
    let mut remaining_length = remaining_length_encode(buffer_size);
    let mut buffer: Vec<u8> = Vec::with_capacity(buffer_size);
    buffer.push(0x10);
    buffer.append(&mut remaining_length);
    buffer.push(0);
    buffer.push(MQTT_VERSION);
    buffer.push(77); // M
    buffer.push(81); // Q
    buffer.push(84); // T
    buffer.push(84); // T
    buffer.push(4); // Protocol Level
    let byte_flags = create_byte_with_flags(&flags, &user_information.will_qos);
    buffer.push(byte_flags); // Connect flags
    buffer.push((user_information.keep_alive >> 8) as u8);
    buffer.push((user_information.keep_alive) as u8);
    buffer.push((user_information.id_length >> 8) as u8);
    buffer.push(user_information.id_length as u8);

    //let mut indice:usize = 14;
    let client_id = user_information.id.as_bytes();
    for byte in client_id.iter() {
        buffer.push(*byte);
    }
    if flags.will_flag {
        buffer.push((user_information.will_topic_length >> 8) as u8);
        buffer.push(user_information.will_topic_length as u8);
        let will_topic = user_information.will_topic.unwrap();
        let will_topic_bytes = will_topic.as_bytes();
        for byte in will_topic_bytes.iter() {
            buffer.push(*byte);
        }
        buffer.push((user_information.will_message_length >> 8) as u8);
        buffer.push(user_information.will_message_length as u8);
        let will_message = user_information.will_message.unwrap();
        let will_message_bytes = will_message.as_bytes();
        for byte in will_message_bytes.iter() {
            buffer.push(*byte);
        }
    }
    if flags.username {
        buffer.push((user_information.username_length >> 8) as u8);
        buffer.push(user_information.username_length as u8);
        let user = user_information.username.unwrap();
        let user_bytes = user.as_bytes();
        for byte in user_bytes.iter() {
            buffer.push(*byte);
        }
    }
    if flags.password {
        buffer.push((user_information.password_length >> 8) as u8);
        buffer.push(user_information.password_length as u8);
        let password = user_information.password.unwrap();
        let password_bytes = password.as_bytes();
        for byte in password_bytes.iter() {
            buffer.push(*byte);
        }
    }

    stream.write_all(&buffer).unwrap();
}

pub fn send_subscribe_packet(stream: &mut TcpStream, topics: Vec<String>, qos: bool) {
    let mut rng = rand::thread_rng();
    let packet_id: u16 = rng.gen();
    let packet_id_left: u8 = (packet_id >> 8) as u8;
    let packet_id_right: u8 = packet_id as u8;

    let mut buffer: Vec<u8> = vec![packet_id_left, packet_id_right];
    for topic in topics.iter() {
        buffer.push((topic.len() >> 8) as u8);
        buffer.push((topic.len() & 0x00FF) as u8);
        buffer.append(&mut topic.as_bytes().to_vec());
        if qos {
            buffer.push(1);
        } else {
            buffer.push(0);
        }
    }

    let mut final_buffer = remaining_length_encode(buffer.len());
    final_buffer.insert(0, Packet::Subscribe.into());
    final_buffer.append(&mut buffer);

    stream.write_all(&final_buffer).unwrap();
}

pub fn send_unsubscribe_packet(stream: &mut TcpStream, topics: Vec<String>) {
    let mut rng = rand::thread_rng();
    let packet_id: u16 = rng.gen();
    let packet_id_left: u8 = (packet_id >> 8) as u8;
    let packet_id_right: u8 = packet_id as u8;
    let mut buffer: Vec<u8> = vec![packet_id_left, packet_id_right];

    for topic in topics.iter() {
        buffer.push((topic.len() >> 8) as u8);
        buffer.push((topic.len() & 0x00FF) as u8);
        buffer.append(&mut topic.as_bytes().to_vec());
    }

    let mut final_buffer = remaining_length_encode(buffer.len());
    final_buffer.insert(0, Packet::Unsubscribe.into());
    final_buffer.append(&mut buffer);

    stream.write_all(&final_buffer).unwrap();
}
///  Sends a publish packet to the server that contains a topic, a message, a dup flag, a qos flag and a retain flag. If QoS is 1 it also includes a packet identifier.
///
pub fn send_publish_packet(
    stream: &mut TcpStream,
    topic: String,
    message: String,
    dup: bool,
    qos: bool,
    retain: bool,
) {
    let mut buffer: Vec<u8> = vec![(topic.len() >> 8) as u8, (topic.len() & 0x00FF) as u8];
    buffer.append(&mut topic.as_bytes().to_vec());

    if qos {
        let mut rng = rand::thread_rng();
        let packet_id: u16 = rng.gen();
        let packet_id_left: u8 = (packet_id >> 8) as u8;
        let packet_id_right: u8 = packet_id as u8;
        buffer.push(packet_id_left);
        buffer.push(packet_id_right);
    }

    buffer.append(&mut message.as_bytes().to_vec());

    let bit_dup: u8 = match dup {
        true => 0x04,
        false => 0x00,
    };

    let retain_byte: u8 = if retain { 0x01 } else { 0 };
    let qos_byte: u8 = if qos { 0x02 } else { 0 };

    let mut final_buffer = remaining_length_encode(buffer.len());

    let mut first_byte = u8::from(Packet::Publish) | bit_dup;
    first_byte |= retain_byte;
    first_byte |= qos_byte;

    final_buffer.insert(0, first_byte);
    final_buffer.append(&mut buffer);

    stream.write_all(&final_buffer).unwrap();
}

pub fn send_pingreq_packet(stream: &mut TcpStream) {
    let buffer = [Packet::PingReq.into(), 0_u8];
    stream.write_all(&buffer).unwrap();
}

pub fn send_puback_packet(stream: &mut TcpStream, packet_identifier: [u8; 2]) {
    let mut buffer = [0u8; 4];
    buffer[0] = Packet::PubAck.into();
    buffer[1] = 0x02;
    buffer[2] = packet_identifier[0];
    buffer[3] = packet_identifier[1];

    stream.write_all(&buffer).unwrap();
}

pub fn _send_disconnect_packet(stream: &mut TcpStream) {
    let buffer = [Packet::Disconnect.into(), 0_u8];
    stream.write_all(&buffer).unwrap();
}
