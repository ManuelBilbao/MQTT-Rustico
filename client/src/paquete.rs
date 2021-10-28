use crate::{
    calcular_longitud_conexion, crear_byte_mediante_flags, FlagsConexion, InformacionUsuario,
};
use std::io::{Read, Write};
use std::net::TcpStream;

const MQTT_VERSION: u8 = 4;

pub enum Paquetes {
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

impl From<u8> for Paquetes {
    fn from(code: u8) -> Self {
        match code & 0xF0 {
            0x10 => Paquetes::Connect,
            0x20 => Paquetes::ConnAck,
            0x30 => Paquetes::Publish,
            0x40 => Paquetes::PubAck,
            0x60 => Paquetes::PubRel,
            0x70 => Paquetes::PubComp,
            0x80 => Paquetes::Subscribe,
            0x90 => Paquetes::SubAck,
            0xA0 => Paquetes::Unsubscribe,
            0xB0 => Paquetes::UnsubAck,
            0xC0 => Paquetes::PingReq,
            0xD0 => Paquetes::PingResp,
            0xE0 => Paquetes::Disconnect,
            _ => Paquetes::Disconnect,
        }
    }
}

impl From<Paquetes> for u8 {
    fn from(paquete: Paquetes) -> Self {
        match paquete {
            Paquetes::Connect => 0x10,
            Paquetes::ConnAck => 0x20,
            Paquetes::Publish => 0x30,
            Paquetes::PubAck => 0x40,
            Paquetes::PubRel => 0x62,
            Paquetes::PubComp => 0x70,
            Paquetes::Subscribe => 0x82,
            Paquetes::SubAck => 0x90,
            Paquetes::Unsubscribe => 0xA2,
            Paquetes::UnsubAck => 0xB0,
            Paquetes::PingReq => 0xC0,
            Paquetes::PingResp => 0xD0,
            Paquetes::Disconnect => 0xE0,
        }
    }
}

pub fn leer_paquete(
    stream: &mut TcpStream,
    tipo_paquete: Paquetes,
    tamanio_lectura: u8,
) -> Result<(), std::io::Error> {
    let mut buffer_paquete: Vec<u8> = vec![0; tamanio_lectura as usize];
    stream.read_exact(&mut buffer_paquete)?;
    match tipo_paquete {
        Paquetes::ConnAck => leer_connack(buffer_paquete),
        Paquetes::PubAck => leer_puback(buffer_paquete),
        Paquetes::SubAck => leer_suback(buffer_paquete),
        Paquetes::UnsubAck => leer_unsuback(buffer_paquete),
        Paquetes::PingResp => leer_pingresp(),
        _ => {
            // Manejar
            println!("No se que paqeute es");
        }
    }
    Ok(())
}
pub fn leer_connack(buffer: Vec<u8>) {
    let session_present = buffer[0];
    let return_code = buffer[1];
    println!(
        "Recibe connack con sp: {} y return code {}",
        session_present, return_code
    );
}

pub fn leer_puback(buffer: Vec<u8>) {
    let packet_identifier = ((buffer[0] as u16) << 8) + buffer[1] as u16;
    println!("Recibido PubAck. Packet Identifier: {}.", packet_identifier);
}

pub fn leer_suback(buffer: Vec<u8>) {
    let packet_identifier = ((buffer[0] as u16) << 8) + buffer[1] as u16;
    println!("Recibido SubAck. Packet Identifier: {}.", packet_identifier);

    let cantidad_topics = buffer.len() - 2;
    let mut topic_results: Vec<u8> = Vec::new();
    print!("Resultados: ");
    for i in 0..cantidad_topics {
        print!("{}, ", &buffer[i + 2]);
        topic_results.push(buffer[i + 2]);
    }
    println!();
}

pub fn leer_unsuback(buffer: Vec<u8>) {
    let packet_identifier = ((buffer[0] as u16) << 8) + buffer[1] as u16;
    println!(
        "Recibido UnsubAck. Packet Identifier: {}.",
        packet_identifier
    );
}

pub fn leer_pingresp() {
    // PingResp viene vacio
    // TODO: Do something
}

pub fn enviar_paquete_conexion(
    stream: &mut TcpStream,
    flags: FlagsConexion,
    informacion_usuario: InformacionUsuario,
) {
    let longitud_total: u8 = calcular_longitud_conexion(&flags, &informacion_usuario);
    let mut buffer_envio: Vec<u8> = Vec::with_capacity(longitud_total.into());
    buffer_envio.push(0x10);
    buffer_envio.push(longitud_total as u8);
    buffer_envio.push(0);
    buffer_envio.push(MQTT_VERSION);
    buffer_envio.push(77); // M
    buffer_envio.push(81); // Q
    buffer_envio.push(84); // T
    buffer_envio.push(84); // T
    buffer_envio.push(4); // Protocol Level
    let byte_flags = crear_byte_mediante_flags(&flags, &informacion_usuario.will_qos);
    buffer_envio.push(byte_flags); // Connect flags
    buffer_envio.push((informacion_usuario.keep_alive >> 8) as u8);
    buffer_envio.push((informacion_usuario.keep_alive) as u8);
    buffer_envio.push((informacion_usuario.longitud_id >> 8) as u8);
    buffer_envio.push(informacion_usuario.longitud_id as u8);

    //let mut indice:usize = 14;
    let client_id = informacion_usuario.id.as_bytes();
    for byte in client_id.iter() {
        buffer_envio.push(*byte);
    }
    if flags.will_flag {
        buffer_envio.push((informacion_usuario.longitud_will_topic >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_will_topic as u8);
        let will_topic = informacion_usuario.will_topic.unwrap();
        let will_topic_bytes = will_topic.as_bytes();
        for byte in will_topic_bytes.iter() {
            buffer_envio.push(*byte);
        }
        buffer_envio.push((informacion_usuario.longitud_will_message >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_will_message as u8);
        let will_message = informacion_usuario.will_message.unwrap();
        let will_message_bytes = will_message.as_bytes();
        for byte in will_message_bytes.iter() {
            buffer_envio.push(*byte);
        }
    }
    if flags.username {
        buffer_envio.push((informacion_usuario.longitud_username >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_username as u8);
        let usuario = informacion_usuario.username.unwrap();
        let usuario_bytes = usuario.as_bytes();
        for byte in usuario_bytes.iter() {
            buffer_envio.push(*byte);
        }
    }
    if flags.password {
        buffer_envio.push((informacion_usuario.longitud_password >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_password as u8);
        let password = informacion_usuario.password.unwrap();
        let password_bytes = password.as_bytes();
        for byte in password_bytes.iter() {
            buffer_envio.push(*byte);
        }
    }
    stream.write_all(&buffer_envio).unwrap();
}

pub fn _enviar_paquete_subscribe(stream: &mut TcpStream, topics: Vec<String>) {
    let mut buffer_envio: Vec<u8> = vec![0x00, 0x00]; // Packet identifier, TODO: parametrizar

    for topic in topics.iter() {
        buffer_envio.push((topic.len() >> 8) as u8);
        buffer_envio.push((topic.len() & 0x00FF) as u8);
        buffer_envio.append(&mut topic.as_bytes().to_vec());
        buffer_envio.push(0x01); // QoS 1, TODO: parametrizar
    }

    buffer_envio.insert(0, buffer_envio.len() as u8); // Remaining length
    buffer_envio.insert(0, Paquetes::Subscribe.into());

    stream.write_all(&buffer_envio).unwrap();
}

pub fn _enviar_paquete_unsubscribe(stream: &mut TcpStream, topics: Vec<String>) {
    let mut buffer_envio: Vec<u8> = vec![0x00, 0x00]; // Packet identifier, TODO: parametrizar

    for topic in topics.iter() {
        buffer_envio.push((topic.len() >> 8) as u8);
        buffer_envio.push((topic.len() & 0x00FF) as u8);
        buffer_envio.append(&mut topic.as_bytes().to_vec());
    }

    buffer_envio.insert(0, buffer_envio.len() as u8); // Remaining length
    buffer_envio.insert(0, Paquetes::Unsubscribe.into());

    stream.write_all(&buffer_envio).unwrap();
}

pub fn _enviar_paquete_publish(stream: &mut TcpStream, topic: String, message: String, dup: bool) {
    let mut buffer_envio: Vec<u8> = vec![(topic.len() >> 8) as u8, (topic.len() & 0x00FF) as u8];
    buffer_envio.append(&mut topic.as_bytes().to_vec());

    buffer_envio.push(0x00); // Packet identifier, TODO: parametrizar
    buffer_envio.push(0x00);

    buffer_envio.append(&mut message.as_bytes().to_vec());

    buffer_envio.insert(0, buffer_envio.len() as u8);

    let bit_dup: u8 = match dup {
        true => 0x04,
        false => 0x00,
    };
    buffer_envio.insert(0, u8::from(Paquetes::Publish) | bit_dup); // TODO: agregar QoS y Retain

    stream.write_all(&buffer_envio).unwrap();
}

pub fn _enviar_paquete_pingreq(stream: &mut TcpStream) {
    let buffer_envio = [Paquetes::PingReq.into(), 0_u8];
    stream.write_all(&buffer_envio).unwrap();
}
