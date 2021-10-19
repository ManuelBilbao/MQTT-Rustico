use crate::{crear_byte_mediante_flags, calcular_longitud_conexion, FlagsConexion, InformacionUsuario};
use std::net::TcpStream;
use std::io::{Read, Write};

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
    DISCONNECT
}

pub fn get_tipo(tipo_leido : u8) -> Paquetes {
    match tipo_leido & 0xF0 {
        0x10 => Paquetes::CONNECT,
        0x20 => Paquetes::CONNACK,
        0x40 => Paquetes::PUBLISH,
        0x80 => Paquetes::SUBSCRIBE,
        0xA0 => Paquetes::UNSUBSCRIBE,
        0xC0 => Paquetes::PINGREQ,
        0xE0 => Paquetes::DISCONNECT,
        _ => Paquetes::DISCONNECT
    }
}

pub fn leer_paquete(stream : &mut TcpStream, tipo_paquete: Paquetes, tamaño_lectura :u8) -> Result<(), std::io::Error>{
    let mut buffer_paquete : Vec<u8> = vec![0; tamaño_lectura as usize];
    stream.read_exact(&mut buffer_paquete)?;
    match tipo_paquete{
        Paquetes::CONNACK =>{
            leer_connack(buffer_paquete);
        },
        _ =>{ // Manejar
        }
    }
    Ok(())
}
pub fn leer_connack(buffer: Vec<u8>){
    let session_present = buffer[0];
    let return_code = buffer[1];
    println!("Recibe connack con sp: {} y return code {}", session_present, return_code);
}

pub fn enviar_paquete_conexion(stream : &mut TcpStream, flags : FlagsConexion, informacion_usuario: InformacionUsuario){
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
    for i in 0..client_id.len(){
        buffer_envio.push(client_id[i]);
    }
    if flags.will_flag {
        buffer_envio.push((informacion_usuario.longitud_will_topic >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_will_topic as u8);
        let will_topic = informacion_usuario.will_topic.unwrap();
        let will_topic_bytes = will_topic.as_bytes();
        for i in 0..will_topic_bytes.len(){
            buffer_envio.push(will_topic_bytes[i]);
        }
        buffer_envio.push((informacion_usuario.longitud_will_message >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_will_message as u8);
        let will_message = informacion_usuario.will_message.unwrap();
        let will_message_bytes = will_message.as_bytes();
        for i in 0..will_message_bytes.len(){
            buffer_envio.push(will_message_bytes[i]);
        }
    }
    if flags.username {
        buffer_envio.push((informacion_usuario.longitud_username >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_username as u8);
        let usuario = informacion_usuario.username.unwrap();
        let usuario_bytes = usuario.as_bytes();
        for i in 0..usuario_bytes.len(){
            buffer_envio.push(usuario_bytes[i]);
        }
    }
    if flags.password {
        buffer_envio.push((informacion_usuario.longitud_password >> 8) as u8);
        buffer_envio.push(informacion_usuario.longitud_password as u8);
        let password = informacion_usuario.password.unwrap();
        let password_bytes = password.as_bytes();
        for i in 0..password_bytes.len(){
            buffer_envio.push(password_bytes[i]);
        }
    }
    stream.write(&buffer_envio).unwrap();
}