use crate::client::Client;
use crate::configuration::Configuration;
use crate::coordinator::run_coordinator;
use crate::packet::{
    inform_client_disconnect_to_coordinator, read_packet, verify_protocol_name,
    verify_version_protocol, Packet,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, Level};
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

pub struct ClientFlags<'a> {
    pub id: usize,
    pub client_id: Option<String>,
    pub connection: &'a mut TcpStream,
    pub sender: Arc<Mutex<Sender<PacketThings>>>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub will_topic: Option<String>,
    pub will_message: Option<String>,
    pub will_qos: u8,
    pub will_retained: bool,
    pub keep_alive: u16,
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
            .with_ansi(false)
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
            .spawn(move || run_coordinator(coordinator_receiver, lock_clients))?;
        Server::wait_new_clients(&address, handler_clients_locks, &mutex_clients_sender)
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

pub fn handle_client(
    id: usize,
    stream: &mut TcpStream,
    client_sender: Arc<Mutex<Sender<PacketThings>>>,
    client_receiver: Receiver<Vec<u8>>,
) {
    let stream_cloned = stream.try_clone().unwrap();
    let mut current_client = ClientFlags {
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
        .spawn(move || send_packets_to_client(client_receiver, stream_cloned))
        .unwrap();

    read_packets_from_client(&mut current_client)
}

fn read_packets_from_client(mut current_client: &mut ClientFlags) {
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
                inform_client_disconnect_to_coordinator(
                    current_client,
                    Vec::new(),
                    Packet::Disconnect,
                );
                println!("El cliente se desconecto y cerro el stream.");
                break;
            }
        }
    }
}

fn send_packets_to_client(client_receiver: Receiver<Vec<u8>>, mut stream_cloned: TcpStream) -> ! {
    loop {
        match client_receiver.recv() {
            Ok(val) => {
                stream_cloned.write_all(&val).unwrap();
            }
            Err(_er) => {}
        }
    }
}

pub fn bytes2string(bytes: &[u8]) -> Result<String, u8> {
    match std::str::from_utf8(bytes) {
        Ok(str) => Ok(str.to_owned()),
        Err(_) => Err(INCORRECT_SERVER_CONNECTION),
    }
}

pub fn make_connection(client: &mut ClientFlags, buffer_packet: Vec<u8>) -> Result<u8, u8> {
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

    if keep_alive > 0 {
        let wait = (f32::from(keep_alive) * 1.5) as u64;
        if client
            .connection
            .set_read_timeout(Some(Duration::new(wait, 0)))
            .is_err()
        {
            error!("Error al establecer el tiempo límite de espera para un cliente")
        }
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
