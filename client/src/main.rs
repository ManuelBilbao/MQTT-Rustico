use crate::packet::{_send_disconnect_packet, read_package, send_packet_connection};
use core::sync::atomic::Ordering;
use std::env::args;
use std::io::Read;
use std::net::{Shutdown, TcpStream};
use std::process::exit;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

mod packet;

static CLIENT_ARGS: usize = 3;

pub struct FlagsConexion {
    username: bool,
    password: bool,
    will_retain: bool,
    will_flag: bool,
    clean_session: bool,
}

pub struct UserInformation {
    id_length: u16,
    id: String,
    username_length: u16,
    username: Option<String>,
    password_length: u16,
    password: Option<String>,
    will_topic_length: u16,
    will_topic: Option<String>,
    will_message_length: u16,
    will_message: Option<String>,
    will_qos: u8,
    keep_alive: u16,
}

fn main() -> Result<(), ()> {
    let argv = args().collect::<Vec<String>>();
    if argv.len() != CLIENT_ARGS {
        println!("Invalid number of arguments");
        let app_name = &argv[0];
        println!("{:?} <host> <port>", app_name);
        return Err(());
    }
    let address = argv[1].clone() + ":" + &argv[2];
    println!("Connecting to... {:?}", address);
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
        username: true,
        password: true,
        will_retain: false,
        will_flag: false,
        clean_session: false,
    };
    let user_information = UserInformation {
        id_length: 1,
        id: "2".to_owned(),
        username_length: 6,
        username: Some("franco".to_owned()),
        password_length: 6,
        password: Some("123pop".to_owned()),
        will_topic_length: 0,
        will_topic: None,
        will_message_length: 0,
        will_message: None,
        will_qos: 1,
        keep_alive: 100,
    };
    send_packet_connection(&mut stream, flags, user_information);
    //thread spawn leer del servidor
    let mut read_stream = stream.try_clone().unwrap();
    let signal = Arc::new(AtomicBool::new(false));
    let signal_clone = signal.clone(); //agregar el .clone(cuando se descomente el disconnect) -> signal.clone();

    let a = thread::spawn(move || {
        loop {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            if signal_clone.load(Ordering::Relaxed) {
                //Cerre el stream
                println!("Ya se cerro el stream!");
                exit(0);
            }
            match read_stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0].into();
                    read_package(&mut read_stream, package_type, num_buffer[1]).unwrap();
                }
                Err(_) => {
                    println!("No se");
                }
            }
        }
    });
    thread::sleep(Duration::from_millis(1000000));
    disconnect(&mut stream, signal);
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

fn create_byte_with_flags(flags: &FlagsConexion, will_qos: &u8) -> u8 {
    let mut byte_flags: u8 = 0;
    if flags.username {
        byte_flags |= 0x80;
    }
    if flags.password {
        byte_flags |= 0x40;
    }
    if flags.will_retain {
        byte_flags |= 0x20;
    }
    byte_flags |= (will_qos << 3) & 0x18;
    if flags.will_flag {
        byte_flags |= 0x04;
    }
    if flags.clean_session {
        byte_flags |= 0x02;
    }
    byte_flags
}

fn calculate_connection_length(flags: &FlagsConexion, user_information: &UserInformation) -> u8 {
    let mut lenght: u8 = 12;
    lenght += user_information.id_length as u8;
    if flags.username {
        lenght += (user_information.username_length + 2) as u8;
    }
    if flags.password {
        lenght += (user_information.password_length + 2) as u8;
    }
    if flags.will_flag {
        lenght +=
            (user_information.will_topic_length + user_information.will_message_length + 4) as u8;
    }
    lenght
}

fn disconnect(stream: &mut TcpStream, signal: Arc<AtomicBool>) {
    _send_disconnect_packet(stream);
    stream
        .shutdown(Shutdown::Both)
        .expect("shutdown call failed");
    signal.store(true, Ordering::Relaxed);
    println!("Me desconecte con exito!");
    exit(0);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sample_client() {
        assert_eq!(1, 1)
    }
}
