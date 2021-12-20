use core::sync::atomic::Ordering;
/*use std::env::args;*/
use interface::run_connection_window;
use std::io::Read;
use std::net::{Shutdown, TcpStream};
use std::process::exit;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use crate::packet::{
    _send_disconnect_packet, read_packet, send_packet_connection, send_pingreq_packet,
};
mod interface;
mod packet;
mod publish_interface;
mod subscription_interface;
use crate::utils::remaining_length_read;

mod utils;

/*static CLIENT_ARGS: usize = 3;*/
/// Flags required for client connection
pub struct FlagsConnection {
    username: bool,
    password: bool,
    will_retain: bool,
    will_flag: bool,
    clean_session: bool,
}
///Information about the client
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
    run_connection_window();
    Ok(())
}
/// Runs client that will send pingreqs, read first 2 bytes from stream when a packet is received and then read the rest of the packet with read_packet
fn client_run(
    mut stream: TcpStream,
    user_information: UserInformation,
    flags: FlagsConnection,
    connack_sender: Sender<String>,
    puback_sender: Sender<String>,
    message_sender: Sender<String>,
    topic_update_sender: Sender<String>,
) -> std::io::Result<()> {
    let keep_alive: u16 = 100;
    send_packet_connection(&mut stream, flags, user_information);
    //thread spawn leer del servidor
    let mut read_stream = stream.try_clone().unwrap();
    let signal = Arc::new(AtomicBool::new(false));
    let signal_clone = signal.clone(); //agregar el .clone(cuando se descomente el disconnect) -> signal.clone();

    let a = thread::spawn(move || {
        let mut pingreq_stream = read_stream.try_clone().unwrap();
        thread::spawn(move || loop {
            send_pingreq_packet(&mut pingreq_stream);
            sleep(Duration::from_secs(keep_alive.into()));
        });
        loop {
            let mut num_buffer = [0u8; 1]; //Recibimos 2 bytes
            if signal_clone.load(Ordering::Relaxed) {
                //Cerre el stream
                exit(0);
            }
            match read_stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let buff_size = remaining_length_read(&mut read_stream).unwrap();
                    read_packet(
                        &mut read_stream,
                        num_buffer[0],
                        buff_size,
                        puback_sender.clone(),
                        message_sender.clone(),
                        connack_sender.clone(),
                        topic_update_sender.clone(),
                    )
                    .unwrap();
                }
                Err(_) => {
                    break;
                }
            }
        }
    });
    thread::sleep(Duration::from_millis(1000000));
    disconnect(&mut stream, signal);
    //ENVIA COSAS al sv
    a.join().unwrap();
    Ok(())
}

fn create_byte_with_flags(flags: &FlagsConnection, will_qos: &u8) -> u8 {
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

fn calculate_connection_length(
    flags: &FlagsConnection,
    user_information: &UserInformation,
) -> usize {
    let mut lenght: usize = 12;
    lenght += user_information.id_length as usize;
    if flags.username {
        lenght += (user_information.username_length as usize) + 2;
    }
    if flags.password {
        lenght += (user_information.password_length as usize) + 2;
    }
    if flags.will_flag {
        lenght += (user_information.will_topic_length as usize)
            + (user_information.will_message_length as usize)
            + 4;
    }
    lenght
}
/// Disconnects client from server by sending disconnect packet and shutting down streams
fn disconnect(stream: &mut TcpStream, signal: Arc<AtomicBool>) {
    _send_disconnect_packet(stream);
    stream
        .shutdown(Shutdown::Both)
        .expect("shutdown call failed");
    signal.store(true, Ordering::Relaxed);
    exit(0);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sample_client() {
        assert_eq!(1, 1)
    }
}
