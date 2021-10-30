use crate::client::Client;
use crate::configuration::Configuration;
use crate::packet::{read_packet, verify_protocol_name, verify_version_protocol, Packet};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use tracing::{debug, info, warn, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

const CONNECTION_IDENTIFIER_REFUSED: u8 = 2;
const _CONNECTION_PROTOCOL_REJECTED: u8 = 1;
const INCORRECT_SERVER_CONNECTION: u8 = 1;
const _SUCCESSFUL_CONNECTION: u8 = 0;

pub struct Server {
    //
    cfg: Configuration,
    _clients: Vec<Client>, //
}

pub struct FlagsCliente<'a> {
    pub id: usize,
    client_id: Option<String>,
    pub connection: &'a mut TcpStream,
    pub sender: Arc<Mutex<Sender<PacketThings>>>,
    username: Option<String>,
    password: Option<String>,
    will_topic: Option<String>,
    will_message: Option<String>,
    will_qos: u8,
    will_retained: bool,
    keep_alive: u16,
}

pub struct PacketThings {
    pub thread_id: usize,
    pub packet_type: Packet,
    pub bytes: Vec<u8>,
}

impl Server {
    pub fn new(file_path: &str) -> Self {
        let mut config = Configuration::new();
        let _aux = config.set_config(file_path); //Manejar
        Server {
            cfg: config,
            _clients: Vec::new(),
        }
    }

    pub fn run(&self) -> std::io::Result<()> {
        let file_appender = RollingFileAppender::new(Rotation::NEVER, "", self.cfg.get_log_file());
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_max_level(Level::TRACE)
            .json()
            .init();
        info!("Arranca sistema de logs.");
        let address = self.cfg.get_address();
        debug!("IP: {}", &address); //
        println!("IP: {}", &address); //

        let clients: Vec<Client> = Vec::new();
        let lock_clients = Arc::new(Mutex::new(clients));
        let handler_clients_locks = lock_clients.clone();
        let (clients_sender, coordinator_receiver): (Sender<PacketThings>, Receiver<PacketThings>) =
            mpsc::channel();
        let mutex_clients_sender = Arc::new(Mutex::new(clients_sender));
        thread::Builder::new()
            .name("Coordinator".into())
            .spawn(move || loop {
                info!("Se lanzado el thread-coordinator");
                match coordinator_receiver.recv() {
                    Ok(mut packet) => {
                        match packet.packet_type {
                            Packet::Subscribe => {
                                info!("Se recibio un paquete Subscribe.");
                                let vector_with_qos =
                                    Server::process_subscribe(&lock_clients, &packet);
                                Server::send_subback(&lock_clients, packet, vector_with_qos)
                            }
                            Packet::Unsubscribe => {
                                info!("Se recibio un paquete Unsubscribe.");
                                Server::unsubscribe_process(&lock_clients, &packet);
                                Server::send_unsubback(&lock_clients, packet)
                            }
                            Packet::Publish => {
                                let topic_name = Server::procesar_publish(&mut packet);
                                if topic_name.is_empty() {
                                    continue;
                                }
                                Server::send_publish_to_customer(
                                    &lock_clients,
                                    &mut packet,
                                    &topic_name,
                                )
                            }
                            _ => {
                                debug!("Se recibio un paquete desconocido.")
                            }
                        }
                        if lock_clients.lock().unwrap().is_empty() {
                            println!("Esta vacio, no lo ves?.");
                            debug!("No hay clientes.");
                        }
                    }
                    Err(_e) => {
                        println!("error del coordinador al recibir un paquete.");
                        warn!("Error del coordinador al recibir un paquete.");
                    }
                }
            })?;

        // Thread Listener
        Server::wait_new_clients(&address, handler_clients_locks, &mutex_clients_sender)
    }

    fn send_publish_to_customer(
        lock_clients: &Arc<Mutex<Vec<Client>>>,
        packet: &mut PacketThings,
        topic_name: &str,
    ) {
        packet.bytes.remove(0);
        match lock_clients.lock() {
            Ok(locked) => {
                for client in locked.iter() {
                    if client.is_subscribed_to(topic_name) {
                        let mut buffer_packet: Vec<u8> =
                            vec![Packet::Publish.into(), packet.bytes.len() as u8];
                        buffer_packet.append(&mut packet.bytes);
                        match client.channel.send(buffer_packet) {
                            Ok(_) => {
                                println!("Publish enviado al cliente")
                            }
                            Err(_) => {
                                println!("Error al enviar Publish al cliente")
                            }
                        }
                    }
                }
            }
            Err(_) => {
                println!("Imposible acceder al lock desde el cordinador")
            }
        }
    }

    fn procesar_publish(packet: &mut PacketThings) -> String {
        let _byte_0 = packet.bytes[0]; //TODO Retained & qos
        let size = packet.bytes.len();
        let topic_name_len: usize = ((packet.bytes[1] as usize) << 8) + packet.bytes[2] as usize;
        let mut topic_name = String::from("");
        match bytes2string(&packet.bytes[3..(3 + topic_name_len)]) {
            Ok(value) => {
                topic_name = value;
            }
            Err(_) => {
                println!("Error procesando topico");
            }
        }
        let mut _topic_desc = String::from(""); //INICIO RETAINED
        if size > 3 + topic_name_len {
            match bytes2string(&packet.bytes[(5 + topic_name_len)..(size)]) {
                Ok(value) => {
                    _topic_desc = value;
                }
                Err(_) => {
                    println!("Error leyendo contenido del publish")
                }
            }
        }
        //TODO: AGREGAR RETAINED ACA
        topic_name
    }

    fn send_unsubback(lock_clients: &Arc<Mutex<Vec<Client>>>, packete: PacketThings) {
        let buffer: Vec<u8> = vec![
            Packet::UnsubAck.into(),
            0x02,
            packete.bytes[0],
            packete.bytes[1],
        ];
        match lock_clients.lock() {
            Ok(locked) => {
                if let Some(index) = locked.iter().position(|r| r.id == packete.thread_id) {
                    match locked[index].channel.send(buffer) {
                        Ok(_) => {
                            println!("SubBack enviado.");
                            info!("SubBack enviado.");
                        }
                        Err(_) => {
                            println!("Error al enviar Unsubback.");
                            debug!("Error con el envio del Unsubback.")
                        }
                    }
                }
            }
            Err(_) => {
                println!("Imposible acceder al lock desde el cordinador.");
                warn!("Imposible acceder al lock desde el cordinador.")
            }
        }
    }

    fn unsubscribe_process(lock_clients: &Arc<Mutex<Vec<Client>>>, packet: &PacketThings) {
        let mut index = 2;
        while index < packet.bytes.len() {
            let topic_size: usize =
                ((packet.bytes[index] as usize) << 8) + packet.bytes[index + 1] as usize;
            index += 2;
            let topico = bytes2string(&packet.bytes[index..(index + topic_size)]).unwrap(); //TODO Cambiar el unwrap
            index += topic_size;
            match lock_clients.lock() {
                Ok(mut locked) => {
                    if let Some(indice) = locked.iter().position(|r| r.id == packet.thread_id) {
                        locked[indice].unsubscribe(topico);
                        info!("El cliente se desuscribio del topico.")
                    }
                }
                Err(_) => {
                    println!("Error al intentar desuscribir de un topico.");
                    warn!("Error al intentar desuscribir de un topico.")
                }
            }
        }
    }

    fn send_subback(
        lock_clientes: &Arc<Mutex<Vec<Client>>>,
        packet: PacketThings,
        vector_with_qos: Vec<u8>,
    ) {
        let mut buffer: Vec<u8> = vec![
            Packet::SubAck.into(),
            (vector_with_qos.len() as u8 + 2_u8) as u8,
            packet.bytes[0],
            packet.bytes[1],
        ];
        for bytes in vector_with_qos {
            buffer.push(bytes);
        }
        match lock_clientes.lock() {
            Ok(locked) => {
                if let Some(index) = locked.iter().position(|r| r.id == packet.thread_id) {
                    match locked[index].channel.send(buffer) {
                        Ok(_) => {
                            println!("SubBack enviado");
                            info!("SubBack enviado")
                        }
                        Err(_) => {
                            println!("Error al enviar Subback");
                            debug!("Error al enviar Subback")
                        }
                    }
                }
            }
            Err(_) => {
                println!("Imposible acceder al lock desde el cordinador");
                warn!("Imposible acceder al lock desde el cordinador")
            }
        }
    }

    fn process_subscribe(lock_clients: &Arc<Mutex<Vec<Client>>>, packet: &PacketThings) -> Vec<u8> {
        let mut index = 2;
        let mut vector_with_qos: Vec<u8> = Vec::new();
        while index < (packet.bytes.len() - 2) {
            let topic_size: usize =
                ((packet.bytes[index] as usize) << 8) + packet.bytes[index + 1] as usize;
            index += 2;
            let topic = bytes2string(&packet.bytes[index..(index + topic_size)]).unwrap();
            index += topic_size;
            let qos: u8 = &packet.bytes[index] & 0x01;
            index += 1;
            match lock_clients.lock() {
                Ok(mut locked) => {
                    if let Some(indice) = locked.iter().position(|r| r.id == packet.thread_id) {
                        locked[indice].subscribe(topic);
                        //TODO: ENVIAR RETAINED SI CORRESPONDE
                        vector_with_qos.push(qos);
                        info!("El cliente se subscribio al topico")
                    }
                }
                Err(_) => vector_with_qos.push(0x80),
            }
        }
        vector_with_qos
    }

    fn wait_new_clients(
        address: &str,
        handler_clients_lock: Arc<Mutex<Vec<Client>>>,
        clients_sender: &Arc<Mutex<Sender<PacketThings>>>,
    ) -> std::io::Result<()> {
        let mut index: usize = 0;
        loop {
            let listener = TcpListener::bind(&address)?;
            let connection = listener.accept()?;
            let mut client_stream: TcpStream = connection.0;
            let (coordinator_sender, client_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
                mpsc::channel();
            let client_sender = Arc::clone(clients_sender);
            let client: Client = Client::new(index, coordinator_sender);
            handler_clients_lock.lock().unwrap().push(client);
            thread::Builder::new()
                .name("Client-Listener".into())
                .spawn(move || {
                    info!("Se lanzo un nuevo cliente.");
                    handle_client(index, &mut client_stream, client_sender, client_receiver);
                })
                .unwrap();
            index += 1;
        }
    }
}

///////////////////////a partir de aca puede ser que lo movamos o renombremos (para mi va aca pero renombrado)

pub fn handle_client(
    id: usize,
    stream: &mut TcpStream,
    client_sender: Arc<Mutex<Sender<PacketThings>>>,
    client_receiver: Receiver<Vec<u8>>,
) {
    let mut stream_cloned = stream.try_clone().unwrap();
    let mut current_client = FlagsCliente {
        id,
        client_id: None,
        connection: stream,
        sender: client_sender,
        username: None,
        password: None,
        will_topic: None,
        will_message: None,
        will_qos: 0,
        will_retained: false,
        keep_alive: 1000,
    };

    thread::Builder::new()
        .name("Client-Communicator".into())
        .spawn(move || loop {
            match client_receiver.recv() {
                Ok(val) => {
                    stream_cloned.write_all(&val).unwrap();
                }
                Err(_er) => {}
            }
        })
        .unwrap();

    loop {
        let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
        match current_client.connection.read_exact(&mut num_buffer) {
            Ok(_) => {
                //Acordarse de leerlo  como BE, let mensaje = u32::from_be_bytes(num_buffer);
                let packet_type = num_buffer[0].into();
                read_packet(
                    &mut current_client,
                    packet_type,
                    num_buffer[1],
                    num_buffer[0],
                )
                .unwrap();
            }
            Err(_) => {
                println!("Error");
                break;
            }
        }
    }
}

pub fn bytes2string(bytes: &[u8]) -> Result<String, u8> {
    match std::str::from_utf8(bytes) {
        Ok(str) => Ok(str.to_owned()),
        Err(_) => Err(INCORRECT_SERVER_CONNECTION),
    }
}

pub fn make_connection(client: &mut FlagsCliente, buffer_packet: Vec<u8>) -> Result<u8, u8> {
    verify_protocol_name(&buffer_packet)?;
    verify_version_protocol(&buffer_packet[6])?;

    let flag_username = buffer_packet[7] & 0x80 == 0x80;
    let flag_password = buffer_packet[7] & 0x40 == 0x40;
    let flag_will_retain = buffer_packet[7] & 0x20 == 0x20;
    let flag_will_qos = (buffer_packet[7] & 0x18) >> 3;
    let flag_will_flag = buffer_packet[7] & 0x04 == 0x04;
    let flag_clean_session = buffer_packet[7] & 0x02 == 0x02;

    let keep_alive: u16 = ((buffer_packet[8] as u16) << 8) + buffer_packet[9] as u16;

    let size_client_id: usize = ((buffer_packet[10] as usize) << 8) + buffer_packet[11] as usize;

    let client_id = Some(bytes2string(&buffer_packet[12..12 + size_client_id])?); // En UTF-8

    let mut index: usize = (12 + size_client_id) as usize;

    // Atajar si tamanio_x = 0
    let mut will_topic = None;
    let mut will_message = None;
    if flag_will_flag {
        let size_will_topic: usize =
            ((buffer_packet[index] as usize) << 8) + buffer_packet[(index + 1)] as usize;
        index += 2_usize;
        will_topic = Some(bytes2string(
            &buffer_packet[index..(index + size_will_topic)],
        )?);
        index += size_will_topic;

        let size_will_message: usize =
            ((buffer_packet[index] as usize) << 8) + buffer_packet[(index + 1) as usize] as usize;
        index += 2_usize;
        will_message = Some(bytes2string(
            &buffer_packet[index..(index + size_will_message)],
        )?);
        index += size_will_message;
    }

    let mut username: Option<String> = None;
    if flag_username {
        let size_username: usize =
            ((buffer_packet[index] as usize) << 8) + buffer_packet[(index + 1) as usize] as usize;
        index += 2_usize;
        username = Some(bytes2string(
            &buffer_packet[index..(index + size_username)],
        )?);
        index += size_username;
    }

    let mut password: Option<String> = None;
    if flag_password {
        let size_password: usize =
            ((buffer_packet[index] as usize) << 8) + buffer_packet[(index + 1) as usize] as usize;
        index += 2_usize;
        password = Some(bytes2string(
            &buffer_packet[index..(index + size_password)],
        )?);
    }

    //PROCESAR
    if client_id == None && !flag_clean_session {
        return Err(CONNECTION_IDENTIFIER_REFUSED);
    }

    if flag_will_retain && flag_will_qos == 2 {
        //
    }

    client.client_id = client_id;
    client.username = username;
    client.password = password;
    client.will_topic = will_topic;
    client.will_message = will_message;
    client.will_qos = flag_will_qos;
    client.will_retained = flag_will_retain;
    client.keep_alive = keep_alive;

    Ok(1) // TODO: Persistent Sessions
}
