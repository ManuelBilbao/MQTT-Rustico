use crate::server::Server;
use std::env::args;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing::Level;

mod client;
mod configuration;
mod coordinator;
mod packet;
mod server;
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
    use std::io::Read;
    use std::io::Write;
    use std::net::TcpStream;
    use std::thread;
    use std::time;

    #[test]
    fn test_01_connect_y_connack_exitoso() {
        //Arrange
        thread::spawn(move || {
            let server = Server::new("src/config.txt");
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
                    match bytes2string(&buffer_paquete[2..(2 + topic_name_len)]) {
                        Ok(topic_name) => {
                            assert_eq!(topic_name, "as/ti/lle/ro");
                        }
                        Err(_) => {
                            panic!("Error leyendo el nombre del tópico en el test 5");
                        }
                    }
                    match bytes2string(&buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()])
                    {
                        Ok(message) => {
                            assert_eq!(message, "piniata");
                        }
                        Err(_) => {
                            panic!("Error leyendo el body del tópico en el test 5");
                        }
                    }
                    can_go_on = true;
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
        let client_id = "24".to_owned();
        let client_id_bytes = client_id.as_bytes();
        for byte in client_id_bytes.iter() {
            buffer.push(*byte);
        }
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
        let client_id = "24".to_owned();
        let client_id_bytes = client_id.as_bytes();
        for byte in client_id_bytes.iter() {
            buffer.push(*byte);
        }
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
                    match bytes2string(&buffer_paquete[2..(2 + topic_name_len)]) {
                        Ok(topic_name) => {
                            assert_eq!(topic_name, "as/tio/lle/ro");
                        }
                        Err(_) => {
                            panic!("Error leyendo el nombre del tópico en el test 5");
                        }
                    }
                    match bytes2string(&buffer_paquete[(4 + topic_name_len)..buffer_paquete.len()])
                    {
                        Ok(message) => {
                            assert_eq!(message, "piniata");
                        }
                        Err(_) => {
                            panic!("Error leyendo el body del tópico en el test 5");
                        }
                    }
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
        let client_id = "24".to_owned();
        let client_id_bytes = client_id.as_bytes();
        for byte in client_id_bytes.iter() {
            buffer.push(*byte);
        }
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
}
