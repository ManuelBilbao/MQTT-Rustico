use crate::{realizar_conexion, realizar_publicacion, Cliente};
use std::io::{Read, Write};

const MQTT_VERSION: u8 = 4;
const MQTT_NAME: [u8; 6] = [0x00, 0x04, 0x4D, 0x51, 0x54, 0x54];
const CONEXION_IDENTIFICADOR_RECHAZADO: u8 = 2;
const CONEXION_PROTOCOLO_RECHAZADO: u8 = 1;
const CONEXION_SERVIDOR_INCORRECTO: u8 = 1;
const CONEXION_EXITOSA: u8 = 0;

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
        0x40 => Paquetes::PUBLISH,
        0x80 => Paquetes::SUBSCRIBE,
        0xA0 => Paquetes::UNSUBSCRIBE,
        0xC0 => Paquetes::PINGREQ,
        0xE0 => Paquetes::DISCONNECT,
        _ => Paquetes::DISCONNECT
    }
}

pub fn leer_paquete(cliente : &mut Cliente, tipo_paquete: Paquetes, tamaño_lectura :u8) -> Result<(), std::io::Error>{
    let mut buffer_paquete : Vec<u8> = vec![0; tamaño_lectura as usize];
    cliente.conexion.read_exact(&mut buffer_paquete)?;
    match tipo_paquete{
        Paquetes::CONNECT =>{
            match realizar_conexion(cliente, buffer_paquete) {
                Ok(session_present) => enviar_resultado_conexion(cliente, CONEXION_EXITOSA, session_present),
                Err(codigo_error) => {
                    enviar_resultado_conexion(cliente, codigo_error, 0);
                }
            }
        },
        Paquetes::PUBLISH =>{
            match realizar_publicacion(buffer_paquete) {
                Ok(_) => {

                }
                Err(_) => {

                }
            }
        },
        Paquetes::SUBSCRIBE => {

        },
        Paquetes::UNSUBSCRIBE => {

        },
        Paquetes::PINGREQ => {

        },
        _ =>{

        }
    }
    
    Ok(())
}

pub fn verificar_nombre_protocolo(buffer: &Vec<u8>) -> Result<(), u8> {
    for i in 0..6 {
        if buffer[i] != MQTT_NAME[i] {
            return Err(CONEXION_SERVIDOR_INCORRECTO);
        }
    }
    Ok(())
}

pub fn verificar_version_protocolo(nivel: &u8) -> Result<(), u8> {
    if *nivel == MQTT_VERSION {
        return Ok(());
    }
    Err(CONEXION_PROTOCOLO_RECHAZADO)
}

fn enviar_resultado_conexion(cliente : &mut Cliente, codigo_resultado : u8, session_present : u8) {
    let mut buffer_envio = [0u8; 4];
    buffer_envio[0] = 0x20;
    buffer_envio[1] = 0x02;
    buffer_envio[2] = session_present;
    buffer_envio[3] = codigo_resultado;

    cliente.conexion.write(&buffer_envio).unwrap();
    println!("Envié el connac");
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test01_nombre_protocolo_correto() {
        let nombre : [u8; 6] = [0x00, 0x04, 0x4D, 0x51, 0x54, 0x54];
        assert_eq!(verificar_nombre_protocolo(&nombre.to_vec()), Ok(()));
    }

    #[test]
    fn test02_nombre_protocolo_incorrecto() {
        let nombre : [u8; 6] = [0x00, 0x04, 0x4E, 0x51, 0x54, 0x14];
        assert_eq!(verificar_nombre_protocolo(&nombre.to_vec()), Err(CONEXION_SERVIDOR_INCORRECTO));
    }

    #[test]
    fn test03_version_protocolo_incorrecta() {
        let version: u8 = 45;
        assert_eq!(verificar_version_protocolo(&version), Err(CONEXION_PROTOCOLO_RECHAZADO));
    }

    #[test]
    fn test04_get_tipo_connect_correcto() {
        let mut rng = rand::thread_rng();
        let header: u8 = 16 + rng.gen_range(0..16);
        let tipo = get_tipo(header);
        assert!(matches!(tipo, Paquetes::CONNECT));
    }

    #[test]
    fn test05_get_tipo_distinto_a_connect() {
        let header: u8 = 32;
        let tipo = get_tipo(header);
        assert_ne!(matches!(tipo, Paquetes::CONNECT), true);
    }
}