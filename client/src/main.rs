use crate::paquete::{enviar_paquete_conexion, leer_paquete};
use std::env::args;
use std::io::Read;
use std::net::TcpStream;
use std::thread;

mod paquete;

static CLIENT_ARGS: usize = 3;

pub struct FlagsConexion {
    username: bool,
    password: bool,
    will_retain: bool,
    will_flag: bool,
    clean_session: bool,
}

pub struct InformacionUsuario {
    longitud_id: u16,
    id: String,
    longitud_username: u16,
    username: Option<String>,
    longitud_password: u16,
    password: Option<String>,
    longitud_will_topic: u16,
    will_topic: Option<String>,
    longitud_will_message: u16,
    will_message: Option<String>,
    will_qos: u8,
    keep_alive: u16,
}

fn main() -> Result<(), ()> {
    let argv = args().collect::<Vec<String>>();
    if argv.len() != CLIENT_ARGS {
        println!("Cantidad de argumentos inválido");
        let app_name = &argv[0];
        println!("{:?} <host> <puerto>", app_name);
        return Err(());
    }
    let address = argv[1].clone() + ":" + &argv[2];
    println!("Conectándome a {:?}", address);
    client_run(&address).unwrap();
    Ok(())
}

fn client_run(address: &str) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(address)?;
    /*let frase = "Hola quiero conectarme".to_string();
    let size_be = (frase.len() as u32).to_be_bytes();
    socket.write(&size_be)?;
    socket.write(&frase.as_bytes())?;*/
    let flags = FlagsConexion {
        username: false,
        password: false,
        will_retain: false,
        will_flag: false,
        clean_session: false,
    };
    let informacion_usuario = InformacionUsuario {
        longitud_id: 1,
        id: "2".to_owned(),
        longitud_username: 0,
        username: None,
        longitud_password: 0,
        password: None,
        longitud_will_topic: 0,
        will_topic: None,
        longitud_will_message: 0,
        will_message: None,
        will_qos: 1,
        keep_alive: 100,
    };
    enviar_paquete_conexion(&mut stream, flags, informacion_usuario);
    //thread spawn leer del servidor
    let mut stream_lectura = stream.try_clone().unwrap();
    let a = thread::spawn(move || {
        loop {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes

            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let tipo_paquete = num_buffer[0].into();
                    leer_paquete(&mut stream_lectura, tipo_paquete, num_buffer[1]).unwrap();
                }
                Err(_) => {
                    println!("No se");
                }
            }
        }
    });
    //ENVIA COSAS al sv
    a.join().unwrap();
    /*
    let mut num_buffer = [0u8; 4];
    socket.read_exact(&mut num_buffer).unwrap();
    let size = u32::from_be_bytes(num_buffer);
    let mut nombre_buf = vec![0; size as usize];
    socket.read_exact(&mut nombre_buf).unwrap();*/
    Ok(())
}

fn crear_byte_mediante_flags(flags: &FlagsConexion, will_qos: &u8) -> u8 {
    let mut byte_flags: u8 = 0;
    if flags.username {
        byte_flags &= 0x80;
    }
    if flags.password {
        byte_flags &= 0x40;
    }
    if flags.will_retain {
        byte_flags &= 0x20;
    }
    byte_flags &= (will_qos << 3) & 0x18;
    if flags.will_flag {
        byte_flags &= 0x04;
    }
    if flags.clean_session {
        byte_flags &= 0x02;
    }
    byte_flags
}

fn calcular_longitud_conexion(
    flags: &FlagsConexion,
    informacion_usuario: &InformacionUsuario,
) -> u8 {
    let mut longitud: u8 = 12;
    longitud += informacion_usuario.longitud_id as u8;
    if flags.username {
        longitud += (informacion_usuario.longitud_username + 2) as u8;
    }
    if flags.password {
        longitud += (informacion_usuario.longitud_password + 2) as u8;
    }
    if flags.will_flag {
        longitud += (informacion_usuario.longitud_will_topic
            + informacion_usuario.longitud_will_message
            + 4) as u8;
    }
    longitud
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sample_client() {
        assert_eq!(1, 1)
    }
}
