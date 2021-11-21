use crate::client::Client;
use crate::configuration::Configuration;
use crate::coordinator::run_coordinator;
use crate::packet::{inform_client_disconnect_to_coordinator, read_packet, Packet};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use tracing::{debug, info, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub struct Server {
    //
    cfg: Configuration,
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
    pub clean_session: u8,
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
        Server { cfg: config }
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

        let clients: HashMap<usize, Client> = HashMap::new();
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
        handler_clients_lock: Arc<Mutex<HashMap<usize, Client>>>,
        clients_sender: &Arc<Mutex<Sender<PacketThings>>>,
    ) -> std::io::Result<()> {
        let mut index: usize = 1;
        loop {
            let listener = TcpListener::bind(&address)?;
            let connection = listener.accept()?;
            let mut client_stream: TcpStream = connection.0;
            let (coordinator_sender, client_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
                mpsc::channel();
            let client_sender = Arc::clone(clients_sender);
            let client: Client = Client::new(index, coordinator_sender);
            handler_clients_lock
                .lock()
                .unwrap()
                .insert(client.thread_id, client);
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
        clean_session: 1,
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
                info!("El cliente se desconecto y cerro el stream."); //
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
