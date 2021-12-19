use crate::server::Server;
use std::env::args;
use tracing::Level;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

mod client;
mod configuration;
mod coordinator;
mod packet;
mod server;
mod stacked_messages;
mod utils;
mod wildcard;

static SERVER_ARGS: usize = 2;

fn main() -> Result<(), ()> {
    let argv = args().collect::<Vec<String>>();
    if argv.len() != SERVER_ARGS {
        println!("Invalid number of arguments");
        return Err(());
    }
    let server = Server::new(&argv[1]);
    let file_appender = RollingFileAppender::new(Rotation::NEVER, "", server.cfg.get_log_file());
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_max_level(Level::TRACE)
        .with_ansi(false)
        .init();
    server.run().unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::bytes2string;
    use crate::utils::{remaining_length_encode, remaining_length_read};
    use std::io::Read;
    use std::io::Write;
    use std::net::{Shutdown, TcpStream};
    use std::sync::mpsc;
    use std::sync::mpsc::{Receiver, Sender};
    use std::thread;
    use std::time;

    #[test]
    fn test_01_connect_y_connack_exitoso() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfg.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1883").unwrap();
        let mut buffer: Vec<u8> = Vec::with_capacity(16);
        buffer.push(0x10); //Connect packet
        buffer.push(14); //Hardcoded length
        buffer.push(0);
        buffer.push(4);
        buffer.push(77); // M
        buffer.push(81); // Q
        buffer.push(84); // T
        buffer.push(84); // T
        buffer.push(4); // Protocol Level
        buffer.push(0); // Connect flags
        buffer.push(0);
        buffer.push(100);
        buffer.push(0);
        buffer.push(2);
        let client_id = "20".to_owned();
        let client_id_bytes = client_id.as_bytes();
        for byte in client_id_bytes.iter() {
            buffer.push(*byte);
        }
        stream.write_all(&buffer).unwrap();
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x20);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    //let session_present = buffer_paquete[0];
                    let return_code = buffer_paquete[1];
                    assert_eq!(return_code, 0);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_02_connect_con_version_incorrecta_devuelve_connack_con_return_code_1() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfga.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1884").unwrap();
        let mut buffer: Vec<u8> = Vec::with_capacity(16);
        buffer.push(0x10); //Connect packet
        buffer.push(14); //Hardcoded length
        buffer.push(0);
        buffer.push(4);
        buffer.push(77); // M
        buffer.push(81); // Q
        buffer.push(84); // T
        buffer.push(84); // T
        buffer.push(3); // Protocol Level
        buffer.push(0); // Connect flags
        buffer.push(0);
        buffer.push(100);
        buffer.push(0);
        buffer.push(2);
        let client_id = "21".to_owned();
        let client_id_bytes = client_id.as_bytes();
        for byte in client_id_bytes.iter() {
            buffer.push(*byte);
        }
        stream.write_all(&buffer).unwrap();
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x20);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    let return_code = buffer_paquete[1];
                    assert_eq!(return_code, 1);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_03_connect_con_userpass_incorrectos_devuelve_connack_con_returncode_4() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgb.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1885").unwrap();
        let mut buffer: Vec<u8> = Vec::with_capacity(22);
        buffer.push(0x10); //Connect packet
        buffer.push(20); //Hardcoded length
        buffer.push(0);
        buffer.push(4);
        buffer.push(77); // M
        buffer.push(81); // Q
        buffer.push(84); // T
        buffer.push(84); // T
        buffer.push(4); // Protocol Level
        buffer.push(0xC0); // Connect flags
        buffer.push(0);
        buffer.push(100);
        buffer.push(0);
        buffer.push(2);
        let client_id = "22".to_owned();
        let client_id_bytes = client_id.as_bytes();
        for byte in client_id_bytes.iter() {
            buffer.push(*byte);
        }
        buffer.push(0);
        buffer.push(1);
        let user = "a".to_owned();
        let user_bytes = user.as_bytes();
        for byte in user_bytes.iter() {
            buffer.push(*byte);
        }
        buffer.push(0);
        buffer.push(1);
        let pass = "a".to_owned();
        let pass_bytes = pass.as_bytes();
        for byte in pass_bytes.iter() {
            buffer.push(*byte);
        }
        stream.write_all(&buffer).unwrap();
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x20);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    let return_code = buffer_paquete[1];
                    assert_eq!(return_code, 4);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_04_connect_sin_clientid_devuelve_connack_con_returncode_2() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgc.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1886").unwrap();
        let mut buffer: Vec<u8> = Vec::with_capacity(14);
        buffer.push(0x10); //Connect packet
        buffer.push(12); //Hardcoded length
        buffer.push(0);
        buffer.push(4);
        buffer.push(77); // M
        buffer.push(81); // Q
        buffer.push(84); // T
        buffer.push(84); // T
        buffer.push(4); // Protocol Level
        buffer.push(0); // Connect flags
        buffer.push(0);
        buffer.push(100);
        buffer.push(0);
        buffer.push(0);
        stream.write_all(&buffer).unwrap();
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x20);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    let return_code = buffer_paquete[1];
                    assert_eq!(return_code, 2);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_05_se_suscribe_publica_y_recibe_ese_publish() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgd.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1887").unwrap();
        connect_and_assert_connection(&mut stream, true);
        //Arrange subscribe packet
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as/ti/#".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange publish
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/ti/lle/ro".to_owned();
        let mut topic_publish_bytes: Vec<u8> = topic_publish.as_bytes().to_vec();
        let body = "piniata".to_owned();
        let mut body_bytes: Vec<u8> = body.as_bytes().to_vec();
        buffer_publish.push(0x32); //Publish code
        buffer_publish.push((4 + topic_publish_bytes.len() + body_bytes.len()) as u8);
        buffer_publish.push(0);
        buffer_publish.push(topic_publish_bytes.len() as u8);
        buffer_publish.append(&mut topic_publish_bytes);
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body_bytes);
        stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck
        let mut first_pubback = true;
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    if package_type == 0x40 {
                        assert_eq!(package_type, 0x40);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        assert_eq!(14, buffer_paquete[1]);
                        can_go_on = true;
                    } else {
                        assert_eq!(package_type & 0xF0, 0x30);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        let topic_name_len: usize = buffer_paquete[1] as usize;
                        assert_eq!(
                            bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                            "as/ti/lle/ro"
                        );
                        assert_eq!(
                            bytes2string(
                                &buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()],
                            ),
                            "piniata"
                        );
                        can_go_on = true;
                        first_pubback = false;
                    }
                }
                Err(_) => {}
            }
        }
        //Assert publish
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    if first_pubback {
                        assert_eq!(package_type & 0xF0, 0x30);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        let topic_name_len: usize = buffer_paquete[1] as usize;
                        assert_eq!(
                            bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                            "as/ti/lle/ro"
                        );
                        assert_eq!(
                            bytes2string(
                                &buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()],
                            ),
                            "piniata"
                        );
                        can_go_on = true;
                    } else {
                        assert_eq!(package_type, 0x40);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        assert_eq!(14, buffer_paquete[1]);
                        can_go_on = true;
                    }
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_06_se_desuscribe_y_no_recibe_ese_publish() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfge.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1888").unwrap();
        connect_and_assert_connection(&mut stream, true);
        //Arrange subscribe packet 1
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as/tio/#".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange subscribe packet 2
        let mut buffer_subscribe_2: Vec<u8> = Vec::new();
        let topic_subscribed_2 = "car/li/#".to_owned();
        let mut topic_subscribed_2_bytes: Vec<u8> = topic_subscribed_2.as_bytes().to_vec();
        buffer_subscribe_2.push(0x80); //Subscribe code
        buffer_subscribe_2.push((5 + topic_subscribed_2_bytes.len()) as u8);
        buffer_subscribe_2.push(0);
        buffer_subscribe_2.push(50);
        buffer_subscribe_2.push(0);
        buffer_subscribe_2.push(topic_subscribed_2_bytes.len() as u8);
        buffer_subscribe_2.append(&mut topic_subscribed_2_bytes);
        buffer_subscribe_2.push(1);
        stream.write_all(&buffer_subscribe_2).unwrap();
        //Assert subscribe exitoso
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(50, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange unsubscribe packet 1
        let mut buffer_unsubscribe: Vec<u8> = Vec::new();
        let topic_unsubscribed = "as/tio/#".to_owned();
        let mut topic_unsubscribed_bytes: Vec<u8> = topic_unsubscribed.as_bytes().to_vec();
        buffer_unsubscribe.push(0xA0); //Unsubscribe code
        buffer_unsubscribe.push((4 + topic_unsubscribed_bytes.len()) as u8);
        buffer_unsubscribe.push(0);
        buffer_unsubscribe.push(51);
        buffer_unsubscribe.push(0);
        buffer_unsubscribe.push(topic_unsubscribed_bytes.len() as u8);
        buffer_unsubscribe.append(&mut topic_unsubscribed_bytes);
        stream.write_all(&buffer_unsubscribe).unwrap();
        //Assert unsubscribe exitoso
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0xB0);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(51, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange publish 1
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/tio/lle/ro".to_owned();
        let mut topic_publish_bytes: Vec<u8> = topic_publish.as_bytes().to_vec();
        let body = "piniata".to_owned();
        let mut body_bytes: Vec<u8> = body.as_bytes().to_vec();
        buffer_publish.push(0x32); //Publish code
        buffer_publish.push((4 + topic_publish_bytes.len() + body_bytes.len()) as u8);
        buffer_publish.push(0);
        buffer_publish.push(topic_publish_bytes.len() as u8);
        buffer_publish.append(&mut topic_publish_bytes);
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body_bytes);
        stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck 1
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x40);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(14, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        thread::sleep(time::Duration::from_millis(20));
        //Arrange publish 2
        let mut buffer_publish_2: Vec<u8> = Vec::new();
        let topic_publish_2 = "car/li/tos".to_owned();
        let mut topic_publish_2_bytes: Vec<u8> = topic_publish_2.as_bytes().to_vec();
        let body_2 = "el vecino".to_owned();
        let mut body_bytes_2: Vec<u8> = body_2.as_bytes().to_vec();
        buffer_publish_2.push(0x32); //Publish code
        buffer_publish_2.push((4 + topic_publish_2_bytes.len() + body_bytes_2.len()) as u8);
        buffer_publish_2.push(0);
        buffer_publish_2.push(topic_publish_2_bytes.len() as u8);
        buffer_publish_2.append(&mut topic_publish_2_bytes);
        buffer_publish_2.push(0);
        buffer_publish_2.push(31); //Packet identifier
        buffer_publish_2.append(&mut body_bytes_2);
        stream.write_all(&buffer_publish_2).unwrap();
        //Assert pubAck 2 & not publish instead
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x40);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(31, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_07_publish_con_retained_envia_mensaje_a_nuevo_suscriptor() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgf.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1889").unwrap();
        connect_and_assert_connection(&mut stream, true);
        //Arrange publish 1
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/tio/lle/ro".to_owned();
        let mut topic_publish_bytes: Vec<u8> = topic_publish.as_bytes().to_vec();
        let body = "piniata".to_owned();
        let mut body_bytes: Vec<u8> = body.as_bytes().to_vec();
        buffer_publish.push(0x33); //Publish code & retain
        buffer_publish.push((4 + topic_publish_bytes.len() + body_bytes.len()) as u8);
        buffer_publish.push(0);
        buffer_publish.push(topic_publish_bytes.len() as u8);
        buffer_publish.append(&mut topic_publish_bytes);
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body_bytes);
        stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x40);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(14, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange subscribe packet 1
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as/tio/#".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Assert publish
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type & 0xF0, 0x30);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    let topic_name_len: usize = buffer_paquete[1] as usize;
                    assert_eq!(
                        bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                        "as/tio/lle/ro"
                    );
                    assert_eq!(
                        bytes2string(&buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()]),
                        "piniata"
                    );
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_08_publish_sin_retained_no_envia_mensaje_a_nuevo_suscriptor() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgg.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1890").unwrap();
        connect_and_assert_connection(&mut stream, true);
        //Arrange publish 1
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/tio/lle/ro".to_owned();
        let mut topic_publish_bytes: Vec<u8> = topic_publish.as_bytes().to_vec();
        let body = "piniata".to_owned();
        let mut body_bytes: Vec<u8> = body.as_bytes().to_vec();
        buffer_publish.push(0x32); //Publish code & retain
        buffer_publish.push((4 + topic_publish_bytes.len() + body_bytes.len()) as u8);
        buffer_publish.push(0);
        buffer_publish.push(topic_publish_bytes.len() as u8);
        buffer_publish.append(&mut topic_publish_bytes);
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body_bytes);
        stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x40);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(14, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange subscribe packet 1
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as/tio/#".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange publish 1
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/ti/lle/ro".to_owned();
        let mut topic_publish_bytes: Vec<u8> = topic_publish.as_bytes().to_vec();
        let body = "piniata".to_owned();
        let mut body_bytes: Vec<u8> = body.as_bytes().to_vec();
        buffer_publish.push(0x33); //Publish code & retain
        buffer_publish.push((4 + topic_publish_bytes.len() + body_bytes.len()) as u8);
        buffer_publish.push(0);
        buffer_publish.push(topic_publish_bytes.len() as u8);
        buffer_publish.append(&mut topic_publish_bytes);
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body_bytes);
        stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck instead of publish retained
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x40);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(14, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_09_suscriptor_con_lastwill_se_desconecta_sin_avisar() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgh.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let (sender, receiver): (Sender<u8>, Receiver<u8>) = mpsc::channel();
        thread::spawn(move || {
            run_client_that_disconnects_ungracefully(receiver);
        });
        thread::sleep(time::Duration::from_millis(100)); //Wait for client to do its things
        let mut stream = TcpStream::connect("127.0.0.1:1891").unwrap();
        connect_and_assert_connection(&mut stream, true);
        //Arrange subscribe packet 1
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arange disconnection of first client
        sender.send(44_u8).unwrap();
        thread::sleep(time::Duration::from_millis(30)); //Wait for client to do its things
                                                        //Assert publish from lastwill
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type & 0xF0, 0x30);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    let topic_name_len: usize = buffer_paquete[1] as usize;
                    assert_eq!(bytes2string(&buffer_paquete[2..(2 + topic_name_len)]), "as");
                    assert_eq!(
                        bytes2string(&buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()]),
                        "pepe"
                    );
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    fn run_client_that_disconnects_ungracefully(receiver: Receiver<u8>) {
        let mut stream = TcpStream::connect("127.0.0.1:1891").unwrap();
        let mut buffer: Vec<u8> = Vec::new();
        buffer.push(0x10); //Connect packet
        buffer.push(24); //Hardcoded length
        buffer.push(0);
        buffer.push(4);
        buffer.push(77); // M
        buffer.push(81); // Q
        buffer.push(84); // T
        buffer.push(84); // T
        buffer.push(4); // Protocol Level
        buffer.push(0x0E); // Connect flags
        buffer.push(0);
        buffer.push(100);
        buffer.push(0);
        buffer.push(2);
        let client_id = "24".to_owned();
        let client_id_bytes = client_id.as_bytes();
        for byte in client_id_bytes.iter() {
            buffer.push(*byte);
        }
        let last_topic = "as".to_owned();
        let mut last_topic_bytes = last_topic.as_bytes().to_vec();
        let last_message = "pepe".to_owned();
        let mut last_message_bytes = last_message.as_bytes().to_vec();
        buffer.push(0);
        buffer.push(last_topic_bytes.len() as u8);
        buffer.append(&mut last_topic_bytes);
        buffer.push(0);
        buffer.push(last_message_bytes.len() as u8);
        buffer.append(&mut last_message_bytes);
        stream.write_all(&buffer).unwrap();
        //Assert connect exitoso
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x20);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    let return_code = buffer_paquete[1];
                    assert_eq!(return_code, 0);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        loop {
            let num = receiver.recv().unwrap();
            if num == 44 {
                break;
            }
        }
    }

    #[test]
    fn test_10_se_suscribe_publica_mensaje_largo_y_recibe_ese_publish() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgi.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1892").unwrap();
        connect_and_assert_connection(&mut stream, true);
        //Arrange subscribe packet
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as/ti/#".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange publish
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/ti/lle/ro".to_owned();
        let topic_len: usize = topic_publish.as_bytes().to_vec().len();
        let body = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Duis tempus metus et lacinia scelerisque. Donec vel est sit amet nulla pellentesque convallis sed sit amet sapien. Donec nec urna posuere, posuere nunc a, bibendum felis. Ut finibus iaculis purus vel imperdiet. Nulla pharetra tristique lacus in hendrerit. Morbi dictum lacus odio, ac fermentum risus efficitur a. Nullam scelerisque sem non dui rhoncus porttitor. Pellentesque quis nibh odio. Praesent eleifend condimentum massa quis luctus. Nunc libero neque, interdum quis scelerisque eget, placerat lacinia sem. Etiam at nunc ac nulla ullamcorper cursus. Pellentesque sollicitudin nisl vitae lorem imperdiet maximus.".to_owned();
        let body_len: usize = body.as_bytes().to_vec().len();
        let mut remaining_length: Vec<u8> = remaining_length_encode(topic_len + body_len + 4);
        buffer_publish.push(0x32); //Publish code
        buffer_publish.append(&mut remaining_length);
        buffer_publish.push(0);
        buffer_publish.push(topic_len as u8);
        buffer_publish.append(&mut topic_publish.as_bytes().to_vec());
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body.as_bytes().to_vec());
        stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck
        let mut first_pubback = true;
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    if package_type == 0x40 {
                        assert_eq!(package_type, 0x40);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        assert_eq!(14, buffer_paquete[1]);
                        can_go_on = true;
                    } else {
                        assert_eq!(package_type & 0xF0, 0x30);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        let topic_name_len: usize = buffer_paquete[1] as usize;
                        assert_eq!(
                            bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                            "as/ti/lle/ro"
                        );
                        assert_eq!(
                            bytes2string(
                                &buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()],
                            ),
                            body
                        );
                        can_go_on = true;
                        first_pubback = false;
                    }
                }
                Err(_) => {}
            }
        }
        thread::sleep(time::Duration::from_millis(20));
        //Assert publish
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 1]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    let buff_size = remaining_length_read(&mut stream).unwrap();
                    if first_pubback {
                        assert_eq!(package_type & 0xF0, 0x30);
                        let mut buffer_paquete: Vec<u8> = vec![0; buff_size as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        let topic_name_len: usize = buffer_paquete[1] as usize;
                        assert_eq!(
                            bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                            topic_publish
                        );
                        assert_eq!(
                            bytes2string(
                                &buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()],
                            ),
                            body
                        );
                        can_go_on = true;
                    } else {
                        assert_eq!(package_type, 0x40);
                        let mut buffer_paquete: Vec<u8> = vec![0; buff_size as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        assert_eq!(14, buffer_paquete[1]);
                        can_go_on = true;
                    }
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_11_como_el_cliente_no_envia_su_pubback_vuelve_a_recibir_el_mensaje() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgj.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1893").unwrap();
        connect_and_assert_connection(&mut stream, true);
        //Arrange subscribe packet
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as/ti/#".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange publish
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/ti/lle/ro".to_owned();
        let topic_len: usize = topic_publish.as_bytes().to_vec().len();
        let body = "hola".to_owned();
        let body_len: usize = body.as_bytes().to_vec().len();
        let mut remaining_length: Vec<u8> = remaining_length_encode(topic_len + body_len + 4);
        buffer_publish.push(0x32); //Publish code
        buffer_publish.append(&mut remaining_length);
        buffer_publish.push(0);
        buffer_publish.push(topic_len as u8);
        buffer_publish.append(&mut topic_publish.as_bytes().to_vec());
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body.as_bytes().to_vec());
        stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck
        let mut first_pubback = true;
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    if package_type == 0x40 {
                        assert_eq!(package_type, 0x40);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        assert_eq!(14, buffer_paquete[1]);
                        can_go_on = true;
                    } else {
                        assert_eq!(package_type & 0xF0, 0x30);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        let topic_name_len: usize = buffer_paquete[1] as usize;
                        assert_eq!(
                            bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                            topic_publish
                        );
                        assert_eq!(
                            bytes2string(
                                &buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()],
                            ),
                            body
                        );
                        can_go_on = true;
                        first_pubback = false;
                    }
                }
                Err(_) => {}
            }
        }
        //Assert publish
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 1]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    let buff_size = remaining_length_read(&mut stream).unwrap();
                    if first_pubback {
                        assert_eq!(package_type & 0xF0, 0x30);
                        let mut buffer_paquete: Vec<u8> = vec![0; buff_size as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        let topic_name_len: usize = buffer_paquete[1] as usize;
                        assert_eq!(
                            bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                            topic_publish
                        );
                        assert_eq!(
                            bytes2string(
                                &buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()],
                            ),
                            body
                        );
                        can_go_on = true;
                    } else {
                        assert_eq!(package_type, 0x40);
                        let mut buffer_paquete: Vec<u8> = vec![0; buff_size as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        assert_eq!(14, buffer_paquete[1]);
                        can_go_on = true;
                    }
                }
                Err(_) => {}
            }
        }
        //Assert publish again

        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 1]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    let buff_size = remaining_length_read(&mut stream).unwrap();
                    assert_eq!(package_type & 0xF0, 0x30);
                    let mut buffer_paquete: Vec<u8> = vec![0; buff_size as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    let topic_name_len: usize = buffer_paquete[1] as usize;
                    assert_eq!(
                        bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                        topic_publish
                    );
                    assert_eq!(
                        bytes2string(&buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()],),
                        body
                    );
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_12_cliente_con_clean_session_no_mantiene_mensajes_encolados_al_desconectarse() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgk.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1894").unwrap();
        connect_and_assert_connection(&mut stream, true);
        //Arrange subscribe packet
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as/ti/#".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange publish
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/ti/lle/ro".to_owned();
        let topic_len: usize = topic_publish.as_bytes().to_vec().len();
        let body = "hola".to_owned();
        let body_len: usize = body.as_bytes().to_vec().len();
        let mut remaining_length: Vec<u8> = remaining_length_encode(topic_len + body_len + 4);
        buffer_publish.push(0x32); //Publish code
        buffer_publish.append(&mut remaining_length);
        buffer_publish.push(0);
        buffer_publish.push(topic_len as u8);
        buffer_publish.append(&mut topic_publish.as_bytes().to_vec());
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body.as_bytes().to_vec());
        stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    if num_buffer[0] != 0x40 {
                        //Publish received so i read and do nothing
                        let buff_size = remaining_length_read(&mut stream).unwrap();
                        let mut buffer_paquete: Vec<u8> = vec![0; buff_size as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                    } else {
                        let package_type = num_buffer[0];
                        assert_eq!(package_type, 0x40);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        assert_eq!(14, buffer_paquete[1]);
                        can_go_on = true;
                    }
                }
                Err(_) => {}
            }
        }
        //Disconnect
        stream
            .shutdown(Shutdown::Both)
            .expect("shutdown call failed");
        let mut new_stream = TcpStream::connect("127.0.0.1:1894").unwrap();
        connect_and_assert_connection(&mut new_stream, true);
        thread::sleep(time::Duration::from_millis(2500)); //Wait for stacked coordinator
                                                          //Arrange new subscribe packet
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "copa".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(51);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        new_stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso instead of publish
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match new_stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    new_stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(51, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_13_cliente_sin_clean_session_mantiene_mensajes_encolados_al_desconectarse() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgl.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1895").unwrap();
        connect_and_assert_connection(&mut stream, false);
        //Arrange subscribe packet
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as/ti/#".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Arrange publish
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/ti/lle/ro".to_owned();
        let topic_len: usize = topic_publish.as_bytes().to_vec().len();
        let body = "hola".to_owned();
        let body_len: usize = body.as_bytes().to_vec().len();
        let mut remaining_length: Vec<u8> = remaining_length_encode(topic_len + body_len + 4);
        buffer_publish.push(0x32); //Publish code
        buffer_publish.append(&mut remaining_length);
        buffer_publish.push(0);
        buffer_publish.push(topic_len as u8);
        buffer_publish.append(&mut topic_publish.as_bytes().to_vec());
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body.as_bytes().to_vec());
        stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    if num_buffer[0] != 0x40 {
                        //Publish received so i read and do nothing
                        let buff_size = remaining_length_read(&mut stream).unwrap();
                        let mut buffer_paquete: Vec<u8> = vec![0; buff_size as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                    } else {
                        let package_type = num_buffer[0];
                        assert_eq!(package_type, 0x40);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        stream.read_exact(&mut buffer_paquete).unwrap();
                        assert_eq!(14, buffer_paquete[1]);
                        can_go_on = true;
                    }
                }
                Err(_) => {}
            }
        }
        //Disconnect
        stream
            .shutdown(Shutdown::Both)
            .expect("shutdown call failed");
        let mut new_stream = TcpStream::connect("127.0.0.1:1895").unwrap();
        connect_and_assert_connection(&mut new_stream, false);
        //Assert publish in queue
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match new_stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type & 0xF0, 0x30);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    new_stream.read_exact(&mut buffer_paquete).unwrap();
                    let topic_name_len: usize = buffer_paquete[1] as usize;
                    assert_eq!(
                        bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                        "as/ti/lle/ro"
                    );
                    assert_eq!(
                        bytes2string(&buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()],),
                        "hola"
                    );
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_14_se_reconecta_habiendo_tenido_clean_session_y_no_guardo_suscripciones() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgm.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1896").unwrap();
        connect_and_assert_connection(&mut stream, true);
        //Arrange subscribe packet
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as/ti/#".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Disconnect
        stream
            .shutdown(Shutdown::Both)
            .expect("shutdown call failed");
        let mut new_stream = TcpStream::connect("127.0.0.1:1896").unwrap();
        connect_and_assert_connection(&mut new_stream, true);
        //Arrange publish 1
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/tio/lle/ro".to_owned();
        let mut topic_publish_bytes: Vec<u8> = topic_publish.as_bytes().to_vec();
        let body = "piniata".to_owned();
        let mut body_bytes: Vec<u8> = body.as_bytes().to_vec();
        buffer_publish.push(0x32); //Publish code
        buffer_publish.push((4 + topic_publish_bytes.len() + body_bytes.len()) as u8);
        buffer_publish.push(0);
        buffer_publish.push(topic_publish_bytes.len() as u8);
        buffer_publish.append(&mut topic_publish_bytes);
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body_bytes);
        new_stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck 1
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match new_stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x40);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    new_stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(14, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        thread::sleep(time::Duration::from_millis(20));
        //Arrange publish 2
        let mut buffer_publish_2: Vec<u8> = Vec::new();
        let topic_publish_2 = "car/li/tos".to_owned();
        let mut topic_publish_2_bytes: Vec<u8> = topic_publish_2.as_bytes().to_vec();
        let body_2 = "el vecino".to_owned();
        let mut body_bytes_2: Vec<u8> = body_2.as_bytes().to_vec();
        buffer_publish_2.push(0x32); //Publish code
        buffer_publish_2.push((4 + topic_publish_2_bytes.len() + body_bytes_2.len()) as u8);
        buffer_publish_2.push(0);
        buffer_publish_2.push(topic_publish_2_bytes.len() as u8);
        buffer_publish_2.append(&mut topic_publish_2_bytes);
        buffer_publish_2.push(0);
        buffer_publish_2.push(31); //Packet identifier
        buffer_publish_2.append(&mut body_bytes_2);
        new_stream.write_all(&buffer_publish_2).unwrap();
        //Assert pubAck 2 & not publish instead
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match new_stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x40);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    new_stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(31, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_15_se_reconecta_sin_haber_tenido_clean_session_y_guardo_suscripciones() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/testingConfigs/cfgn.txt");
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(20)); //Wait for server to start
        let mut stream = TcpStream::connect("127.0.0.1:1897").unwrap();
        connect_and_assert_connection(&mut stream, false);
        //Arrange subscribe packet
        let mut buffer_subscribe: Vec<u8> = Vec::new();
        let topic_subscribed = "as/ti/#".to_owned();
        let mut topic_subscribed_bytes: Vec<u8> = topic_subscribed.as_bytes().to_vec();
        buffer_subscribe.push(0x80); //Subscribe code
        buffer_subscribe.push((5 + topic_subscribed_bytes.len()) as u8);
        buffer_subscribe.push(0);
        buffer_subscribe.push(57);
        buffer_subscribe.push(0);
        buffer_subscribe.push(topic_subscribed_bytes.len() as u8);
        buffer_subscribe.append(&mut topic_subscribed_bytes);
        buffer_subscribe.push(1);
        stream.write_all(&buffer_subscribe).unwrap();
        //Assert subscribe exitoso
        let mut can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x90);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    assert_eq!(57, buffer_paquete[1]);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
        //Disconnect
        stream
            .shutdown(Shutdown::Both)
            .expect("shutdown call failed");
        let mut new_stream = TcpStream::connect("127.0.0.1:1897").unwrap();
        connect_and_assert_connection(&mut new_stream, true);
        //Arrange publish
        let mut buffer_publish: Vec<u8> = Vec::new();
        let topic_publish = "as/ti/lle/ro".to_owned();
        let topic_len: usize = topic_publish.as_bytes().to_vec().len();
        let body = "hola".to_owned();
        let body_len: usize = body.as_bytes().to_vec().len();
        let mut remaining_length: Vec<u8> = remaining_length_encode(topic_len + body_len + 4);
        buffer_publish.push(0x32); //Publish code
        buffer_publish.append(&mut remaining_length);
        buffer_publish.push(0);
        buffer_publish.push(topic_len as u8);
        buffer_publish.append(&mut topic_publish.as_bytes().to_vec());
        buffer_publish.push(0);
        buffer_publish.push(14); //Packet identifier
        buffer_publish.append(&mut body.as_bytes().to_vec());
        new_stream.write_all(&buffer_publish).unwrap();
        //Assert pubAck
        let mut first_pubback = true;
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match new_stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    if package_type == 0x40 {
                        assert_eq!(package_type, 0x40);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        new_stream.read_exact(&mut buffer_paquete).unwrap();
                        assert_eq!(14, buffer_paquete[1]);
                        can_go_on = true;
                    } else {
                        assert_eq!(package_type & 0xF0, 0x30);
                        let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                        new_stream.read_exact(&mut buffer_paquete).unwrap();
                        let topic_name_len: usize = buffer_paquete[1] as usize;
                        assert_eq!(
                            bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                            topic_publish
                        );
                        assert_eq!(
                            bytes2string(
                                &buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()],
                            ),
                            body
                        );
                        can_go_on = true;
                        first_pubback = false;
                    }
                }
                Err(_) => {}
            }
        }
        //Assert publish
        can_go_on = false;
        while !can_go_on {
            let mut num_buffer = [0u8; 1]; //Recibimos 2 bytes
            match new_stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    let buff_size = remaining_length_read(&mut new_stream).unwrap();
                    if first_pubback {
                        assert_eq!(package_type & 0xF0, 0x30);
                        let mut buffer_paquete: Vec<u8> = vec![0; buff_size as usize];
                        new_stream.read_exact(&mut buffer_paquete).unwrap();
                        let topic_name_len: usize = buffer_paquete[1] as usize;
                        assert_eq!(
                            bytes2string(&buffer_paquete[2..(2 + topic_name_len)]),
                            topic_publish
                        );
                        assert_eq!(
                            bytes2string(
                                &buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()],
                            ),
                            body
                        );
                        can_go_on = true;
                    } else {
                        assert_eq!(package_type, 0x40);
                        let mut buffer_paquete: Vec<u8> = vec![0; buff_size as usize];
                        new_stream.read_exact(&mut buffer_paquete).unwrap();
                        assert_eq!(14, buffer_paquete[1]);
                        can_go_on = true;
                    }
                }
                Err(_) => {}
            }
        }
    }

    fn connect_and_assert_connection(stream: &mut TcpStream, clean_session: bool) {
        let mut buffer: Vec<u8> = Vec::with_capacity(16);
        buffer.push(0x10); //Connect packet
        buffer.push(14); //Hardcoded length
        buffer.push(0);
        buffer.push(4);
        buffer.push(77); // M
        buffer.push(81); // Q
        buffer.push(84); // T
        buffer.push(84); // T
        buffer.push(4); // Protocol Level
        if clean_session {
            buffer.push(2); // Connect flags
        } else {
            buffer.push(0);
        }
        buffer.push(0);
        buffer.push(200);
        buffer.push(0);
        buffer.push(2);
        let client_id = "23".to_owned();
        let client_id_bytes = client_id.as_bytes();
        for byte in client_id_bytes.iter() {
            buffer.push(*byte);
        }
        stream.write_all(&buffer).unwrap();
        let mut can_go_on = false;
        //Assert connect exitoso
        while !can_go_on {
            let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
            match stream.read_exact(&mut num_buffer) {
                Ok(_) => {
                    let package_type = num_buffer[0];
                    assert_eq!(package_type, 0x20);
                    let mut buffer_paquete: Vec<u8> = vec![0; num_buffer[1] as usize];
                    stream.read_exact(&mut buffer_paquete).unwrap();
                    let return_code = buffer_paquete[1];
                    assert_eq!(return_code, 0);
                    can_go_on = true;
                }
                Err(_) => {}
            }
        }
    }
}
