use crate::server::{realizar_conexion, FlagsCliente, Paquete};
use std::io::{Read, Write};

const MQTT_VERSION: u8 = 4;
const MQTT_NAME: [u8; 6] = [0x00, 0x04, 0x4D, 0x51, 0x54, 0x54];
const _CONEXION_IDENTIFICADOR_RECHAZADO: u8 = 2;
const CONEXION_PROTOCOLO_RECHAZADO: u8 = 1;
const CONEXION_SERVIDOR_INCORRECTO: u8 = 1;
const CONEXION_EXITOSA: u8 = 0;

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
            0x80 => Paquetes::Subscribe,
            0xA0 => Paquetes::Unsubscribe,
            0xC0 => Paquetes::PingReq,
            0xE0 => Paquetes::Disconnect,
            _ => Paquetes::Disconnect,
        }
    }
}

impl From<Paquetes> for u8 {
    fn from(code: Paquetes) -> Self {
        match code {
            Paquetes::Connect => 0x10,
            Paquetes::ConnAck => 0x20,
            Paquetes::Publish => 0x30,
            Paquetes::Subscribe => 0x82,
            Paquetes::Unsubscribe => 0xA2,
            Paquetes::PingReq => 0xC0,
            Paquetes::Disconnect => 0xE0,
            _ => 0xE0,
        }
    }
}

pub fn leer_paquete(
    cliente: &mut FlagsCliente,
    tipo_paquete: Paquetes,
    tamanio_lectura: u8,
) -> Result<(), std::io::Error> {
    let mut buffer_paquete: Vec<u8> = vec![0; tamanio_lectura as usize];
    cliente.conexion.read_exact(&mut buffer_paquete)?;
    match tipo_paquete {
        Paquetes::Connect => {
            println!("Recibido paquete Connect");
            match realizar_conexion(cliente, buffer_paquete) {
                Ok(session_present) => {
                    enviar_resultado_conexion(cliente, CONEXION_EXITOSA, session_present)
                }
                Err(codigo_error) => {
                    enviar_resultado_conexion(cliente, codigo_error, 0);
                }
            }
        }
        Paquetes::Publish => {
            println!("Recibido paquete Publish");
            /*match realizar_publicacion(buffer_paquete) {
                Ok(_) => {
                }
                Err(_) => {
                }
            }*/
        }
        Paquetes::Subscribe =>  {
            cambiar_suscripcion(cliente, buffer_paquete, Paquetes::Subscribe);
        },
        Paquetes::Unsubscribe => {
            cambiar_suscripcion(cliente, buffer_paquete, Paquetes::Unsubscribe);
        }
        Paquetes::PingReq => {
            println!("Recibido paquete Pinreq");
        }
        _ => {
            println!("Recibido paquete desconocido");
        }
    }

    Ok(())
}

fn cambiar_suscripcion(cliente: &mut FlagsCliente, buffer_paquete: Vec<u8>, tipo: Paquetes){
    let paquete_a_servidor = Paquete{
        thread_id: cliente.id,
        packet_type: tipo,
        bytes: buffer_paquete
    };
    let sender = cliente.sender.lock();
    match sender {
        Ok(sender_ok) => {
            match sender_ok.send(paquete_a_servidor) {
                Ok(_) => { println!("Exito enviando cambio de subscripcion al thread coordinador") },
                Err(_) => { println!("Error enviando cambio de subscripcion al thread coordinador") }
            };
        },
        Err(_) => {println!("Error al leer cambio de suscripcion")}
    }
}

pub fn verificar_nombre_protocolo(buffer: &[u8]) -> Result<(), u8> {
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

fn enviar_resultado_conexion(
    cliente: &mut FlagsCliente,
    codigo_resultado: u8,
    session_present: u8,
) {
    let mut buffer_envio = [0u8; 4];
    buffer_envio[0] = 0x20;
    buffer_envio[1] = 0x02;
    buffer_envio[2] = session_present;
    buffer_envio[3] = codigo_resultado;

    cliente.conexion.write_all(&buffer_envio).unwrap();
    println!("Envié el connac");
}

fn _realizar_publicacion(_buffer_paquete: Vec<u8>) -> Result<(), String> {
    Err("error".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test01_nombre_protocolo_correto() {
        let nombre: [u8; 6] = [0x00, 0x04, 0x4D, 0x51, 0x54, 0x54];
        assert_eq!(verificar_nombre_protocolo(&nombre.to_vec()), Ok(()));
    }

    #[test]
    fn test02_nombre_protocolo_incorrecto() {
        let nombre: [u8; 6] = [0x00, 0x04, 0x4E, 0x51, 0x54, 0x14];
        assert_eq!(
            verificar_nombre_protocolo(&nombre.to_vec()),
            Err(CONEXION_SERVIDOR_INCORRECTO)
        );
    }

    #[test]
    fn test03_version_protocolo_incorrecta() {
        let version: u8 = 45;
        assert_eq!(
            verificar_version_protocolo(&version),
            Err(CONEXION_PROTOCOLO_RECHAZADO)
        );
    }

    #[test]
    fn test04_get_tipo_connect_correcto() {
        let mut rng = rand::thread_rng();
        let header: u8 = 16 + rng.gen_range(0..16);
        let tipo = header.into();
        assert!(matches!(tipo, Paquetes::Connect));
    }

    #[test]
    fn test05_get_tipo_distinto_a_connect() {
        let header: u8 = 32;
        let tipo = header.into();
        assert_ne!(matches!(tipo, Paquetes::Connect), true);
    }
}
