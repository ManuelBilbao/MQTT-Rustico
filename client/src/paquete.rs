use crate::{
    calcular_longitud_conexion, crear_byte_mediante_flags, FlagsConexion, InformacionUsuario,
};
use std::io::{Read, Write};
use std::net::TcpStream;

const MQTT_VERSION: u8 = 4;

pub enum Paquetes {
    CONNECT,
    CONNACK,
    PUBLISH,
    PUBACK,
    PUBREL,
    PUBCOMP,
    SUBSCRIBE,
    SUBACK,
    UNSUBSCRIBE,
    UNSUBACK,
    PINGREQ,
    PINGRESP,
    DISCONNECT,
}

impl From<u8> for Paquetes {
    fn from(code: u8) -> Self {
        match code & 0xF0 {
            0x10 => Paquetes::CONNECT,
            0x20 => Paquetes::CONNACK,
            0x30 => Paquetes::PUBLISH,
            0x80 => Paquetes::SUBSCRIBE,
            0xA0 => Paquetes::UNSUBSCRIBE,
            0xC0 => Paquetes::PINGREQ,
            0xE0 => Paquetes::DISCONNECT,
            _ => Paquetes::DISCONNECT,
        }
    }
}

impl From<Paquetes> for u8 {
    fn from(paquete: Paquetes) -> Self {
        match paquete {
            Paquetes::CONNECT => 0x10,
            Paquetes::CONNACK => 0x20,
            Paquetes::PUBLISH => 0x30,
            Paquetes::SUBSCRIBE => 0x82,
            Paquetes::UNSUBSCRIBE => 0xA2,
            Paquetes::PINGREQ => 0xC0,
            Paquetes::DISCONNECT => 0xE0,
            _ => 0xE0,
        }
    }
}

pub fn leer_paquete(
    stream: &mut TcpStream,
    tipo_paquete: Paquetes,
    tamaño_lectura: u8,
) -> Result<(), std::io::Error> {
    let mut buffer_paquete: Vec<u8> = vec![0; tamaño_lectura as usize];
    stream.read_exact(&mut buffer_paquete)?;
    match tipo_paquete {
        Paquetes::CONNACK => {
            leer_connack(buffer_paquete);
        }
        Paquetes::SUBACK => {
            // Manejar SUBACK
        }
        _ => { // Manejar
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
    for byte in client_id {
        buffer_envio.push(*byte);
    }
    if flags.will_flag {
        buffer_envio.push((informacion_usuario.longitud_will_topic >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_will_topic as u8);
        let will_topic = informacion_usuario.will_topic.unwrap();
        let will_topic_bytes = will_topic.as_bytes();
        for byte in will_topic_bytes {
            buffer_envio.push(*byte);
        }
        buffer_envio.push((informacion_usuario.longitud_will_message >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_will_message as u8);
        let will_message = informacion_usuario.will_message.unwrap();
        let will_message_bytes = will_message.as_bytes();
        for byte in will_message_bytes {
            buffer_envio.push(*byte);
        }
    }
    if flags.username {
        buffer_envio.push((informacion_usuario.longitud_username >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_username as u8);
        let usuario = informacion_usuario.username.unwrap();
        let usuario_bytes = usuario.as_bytes();
        for byte in usuario_bytes {
            buffer_envio.push(*byte);
        }
    }
    if flags.password {
        buffer_envio.push((informacion_usuario.longitud_password >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_password as u8);
        let password = informacion_usuario.password.unwrap();
        let password_bytes = password.as_bytes();
        for byte in password_bytes {
            buffer_envio.push(*byte);
        }
    }
    stream.write_all(&buffer_envio).unwrap();
}

pub fn enviar_paquete_suscribe(stream: &mut TcpStream, topics: Vec<String>) {
    let mut buffer_envio: Vec<u8> = vec![0x00, 0x00]; // Packet identifier, TODO: parametrizar

    for topic in topics.iter() {
        buffer_envio.push((topic.len() >> 8) as u8);
        buffer_envio.push((topic.len() & 0x00FF) as u8);
        buffer_envio.append(&mut topic.as_bytes().to_vec());
        buffer_envio.push(0x01); // QoS 1, TODO: parametrizar
    }

    buffer_envio.insert(0, buffer_envio.len() as u8); // Remaining length
    buffer_envio.insert(0, Paquetes::SUBSCRIBE.into());

    stream.write_all(&buffer_envio).unwrap();
}

pub fn enviar_paquete_unsuscribe(stream: &mut TcpStream, topics: Vec<String>) {
    let mut buffer_envio: Vec<u8> = vec![0x00, 0x00]; // Packet identifier, TODO: parametrizar

    for topic in topics.iter() {
        buffer_envio.push((topic.len() >> 8) as u8);
        buffer_envio.push((topic.len() & 0x00FF) as u8);
        buffer_envio.append(&mut topic.as_bytes().to_vec());
    }

    buffer_envio.insert(0, buffer_envio.len() as u8); // Remaining length
    buffer_envio.insert(0, Paquetes::UNSUBSCRIBE.into());

    stream.write_all(&buffer_envio).unwrap();
}