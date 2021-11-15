use crate::{calculate_connection_length, create_byte_with_flags, FlagsConexion, UserInformation};
use std::io::{Read, Write};
use std::net::TcpStream;

const MQTT_VERSION: u8 = 4;

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

pub fn read_package(
    stream: &mut TcpStream,
    package_type: Packet,
    buffer: u8,
) -> Result<(), std::io::Error> {
    let mut buffer_paquete: Vec<u8> = vec![0; buffer as usize];
    let mut stream_clone = stream.try_clone().unwrap();
    stream.read_exact(&mut buffer_paquete)?;
    match package_type {
        Packet::ConnAck => read_connack(buffer_paquete),
        Packet::PubAck => read_puback(buffer_paquete),
        Packet::SubAck => read_suback(buffer_paquete),
        Packet::UnsubAck => read_unsuback(buffer_paquete),
        Packet::PingResp => read_pingresp(),
        Packet::Publish => read_publish(buffer_paquete, &mut stream_clone),
        _ => {
            // Manejar
            println!("No se que paqeute es");
        }
    }
    Ok(())
}
pub fn read_connack(buffer: Vec<u8>) {
    let session_present = buffer[0];
    let return_code = buffer[1];
    println!(
        "Recibe connack con sp: {} y return code {}",
        session_present, return_code
    );
}

pub fn read_puback(buffer: Vec<u8>) {
    let packet_identifier = ((buffer[0] as u16) << 8) + buffer[1] as u16;
    println!("Recibido PubAck. Packet Identifier: {}.", packet_identifier);
}

pub fn read_suback(buffer: Vec<u8>) {
    let packet_identifier = ((buffer[0] as u16) << 8) + buffer[1] as u16;
    println!("Recibido SubAck. Packet Identifier: {}.", packet_identifier);

    let amount_of_topics = buffer.len() - 2;
    let mut topic_results: Vec<u8> = Vec::new();
    print!("Resultados: ");
    for i in 0..amount_of_topics {
        print!("{}, ", &buffer[i + 2]);
        topic_results.push(buffer[i + 2]);
    }
    println!();
}

pub fn read_unsuback(buffer: Vec<u8>) {
    let packet_identifier = ((buffer[0] as u16) << 8) + buffer[1] as u16;
    println!(
        "Recibido UnsubAck. Packet Identifier: {}.",
        packet_identifier
    );
}

pub fn read_pingresp() {
    // PingResp viene vacio
    // TODO: Do something
}
pub fn read_publish(buffer: Vec<u8>, stream: &mut TcpStream) {
    println!("Recibí un mensaje");
    let topic_name_len: usize = ((buffer[0] as usize) << 8) + buffer[1] as usize;
    match bytes2string(&buffer[2..(2 + topic_name_len)]) {
        Ok(topic_name) => {
            println!("Topico del mensaje: {}", topic_name);
        }
        Err(_) => {
            println!("Error procesando topico");
        }
    }
    match bytes2string(&buffer[(4 + topic_name_len)..buffer.len()]) {
        Ok(message) => {
            println!("Mensaje: {}", message);
        }
        Err(_) => {
            println!("Error procesando mensaje");
        }
    }
    let mut packet_identifier = [0u8; 2];
    packet_identifier[0] = buffer[topic_name_len + 2];
    packet_identifier[1] = buffer[topic_name_len + 3];
    send_puback_packet(stream, packet_identifier);
}

pub fn bytes2string(bytes: &[u8]) -> Result<String, u8> {
    match std::str::from_utf8(bytes) {
        Ok(str) => Ok(str.to_owned()),
        Err(_) => Err(1),
    }
}

pub fn send_packet_connection(
    stream: &mut TcpStream,
    flags: FlagsConexion,
    user_information: UserInformation,
) {
    let total_lenght: u8 = calculate_connection_length(&flags, &user_information);
    let mut buffer: Vec<u8> = Vec::with_capacity(total_lenght.into());
    buffer.push(0x10);
    buffer.push(total_lenght as u8);
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

pub fn _send_subscribe_packet(stream: &mut TcpStream, topics: Vec<String>) {
    let mut buffer: Vec<u8> = vec![0x00, 0x00]; // Packet identifier, TODO: parametrizar

    for topic in topics.iter() {
        buffer.push((topic.len() >> 8) as u8);
        buffer.push((topic.len() & 0x00FF) as u8);
        buffer.append(&mut topic.as_bytes().to_vec());
        buffer.push(0x01); // QoS 1, TODO: parametrizar
    }

    buffer.insert(0, buffer.len() as u8); // Remaining length
    buffer.insert(0, Packet::Subscribe.into());

    stream.write_all(&buffer).unwrap();
}

pub fn _send_unsubscribe_packet(stream: &mut TcpStream, topics: Vec<String>) {
    let mut buffer: Vec<u8> = vec![0x00, 0x00]; // Packet identifier, TODO: parametrizar

    for topic in topics.iter() {
        buffer.push((topic.len() >> 8) as u8);
        buffer.push((topic.len() & 0x00FF) as u8);
        buffer.append(&mut topic.as_bytes().to_vec());
    }

    buffer.insert(0, buffer.len() as u8); // Remaining length
    buffer.insert(0, Packet::Unsubscribe.into());

    stream.write_all(&buffer).unwrap();
}

pub fn _send_publish_packet(stream: &mut TcpStream, topic: String, message: String, dup: bool) {
    let mut buffer: Vec<u8> = vec![(topic.len() >> 8) as u8, (topic.len() & 0x00FF) as u8];
    buffer.append(&mut topic.as_bytes().to_vec());

    buffer.push(0x01); // Packet identifier, TODO: parametrizar
    buffer.push(0x02);

    buffer.append(&mut message.as_bytes().to_vec());

    buffer.insert(0, buffer.len() as u8);

    let bit_dup: u8 = match dup {
        true => 0x04,
        false => 0x00,
    };
    buffer.insert(0, u8::from(Packet::Publish) | bit_dup); // TODO: agregar QoS y Retain

    stream.write_all(&buffer).unwrap();
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

    println!("envié puback");
    stream.write_all(&buffer).unwrap();
}

pub fn _send_disconnect_packet(stream: &mut TcpStream) {
    let buffer = [Packet::Disconnect.into(), 0_u8];
    stream.write_all(&buffer).unwrap();
}
