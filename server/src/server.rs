use crate::client::Client;
use crate::configuration::Configuration;
use crate::coordinator::run_coordinator;
use crate::packet::{inform_client_disconnect_to_coordinator, read_packet, Packet};
use crate::stacked_messages::run_stacked_coordinator;
use crate::utils::remaining_length_read;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use tracing::{debug, error, info, warn};

pub struct Server {
    //
    pub cfg: Configuration,
}

pub struct ClientFlags<'a> {
    pub id: usize,
    pub client_id: Option<String>,
    pub connection: &'a mut TcpStream,
    pub sender: Arc<Mutex<Sender<PacketThings>>>,
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
        config.set_config(file_path).unwrap(); // Si la configuracion es invalida, no puede arrancar el servidor, asi que paniqueo
        Server { cfg: config }
    }

    /// Launchs the main server structures, Coordinator and Stacked messages coordinator and calls the fuctions for connect new clients
    ///
    pub fn run(&self) -> std::io::Result<()> {
        info!("Log system started");
        let address = self.cfg.get_address();
        debug!("IP: {}", &address); //
        println!("IP: {}", &address); //

        let clients: HashMap<usize, Client> = HashMap::new();
        let lock_clients = Arc::new(Mutex::new(clients));
        let handler_clients_locks = lock_clients.clone();
        let lock_clients_stacked_messages = lock_clients.clone();
        let (clients_sender, coordinator_receiver): (Sender<PacketThings>, Receiver<PacketThings>) =
            mpsc::channel();
        let mutex_clients_sender = Arc::new(Mutex::new(clients_sender));
        let password_required = self.cfg.password;
        thread::Builder::new()
            .name("Coordinator".into())
            .spawn(move || run_coordinator(coordinator_receiver, lock_clients))?;
        thread::Builder::new()
            .name("Stacked messages coordinator".into())
            .spawn(move || run_stacked_coordinator(lock_clients_stacked_messages))?;
        Server::wait_new_clients(
            &address,
            handler_clients_locks,
            &mutex_clients_sender,
            password_required,
        )
    }

    /// Waiting for customers, when one appears, launch a new thread to handle it and keeps in loop waiting more new customers.
    ///
    fn wait_new_clients(
        address: &str,
        handler_clients_lock: Arc<Mutex<HashMap<usize, Client>>>,
        clients_sender: &Arc<Mutex<Sender<PacketThings>>>,
        password_required: bool,
    ) -> std::io::Result<()> {
        let mut index: usize = 1;
        loop {
            let listener = TcpListener::bind(&address)?;
            let connection = listener.accept()?;
            let mut client_stream: TcpStream = connection.0;
            let (coordinator_sender, client_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
                mpsc::channel();
            let client_sender_1 = Arc::clone(clients_sender);
            let client_sender_2 = Arc::clone(clients_sender);
            let client: Client = Client::new(index, coordinator_sender);
            match handler_clients_lock.lock() {
                Ok(mut locked) => {
                    locked.insert(client.thread_id, client);
                }
                Err(_) => {
                    error!("Error adding new client");
                }
            }
            match thread::Builder::new()
                .name("Client-Listener".into())
                .spawn(move || {
                    handle_client(
                        index,
                        &mut client_stream,
                        client_sender_1,
                        client_sender_2,
                        client_receiver,
                        password_required,
                    );
                }) {
                Ok(_) => {
                    info!("New client thread");
                }
                Err(_) => {
                    error!("Error running a new client thread");
                }
            }
            index += 1;
        }
    }
}
/// Launchs the Client-Communicator and a function that wait for client messages in a loop.
///
pub fn handle_client(
    id: usize,
    stream: &mut TcpStream,
    client_sender_1: Arc<Mutex<Sender<PacketThings>>>,
    client_sender_2: Arc<Mutex<Sender<PacketThings>>>,
    client_receiver: Receiver<Vec<u8>>,
    password_required: bool,
) {
    let stream_cloned = stream.try_clone().unwrap(); // Si no puede clonar, paniqueo para cerrar el thread Client-Listener
    let mut current_client = ClientFlags {
        id,
        client_id: None,
        connection: stream,
        sender: client_sender_1,
        clean_session: 1,
        keep_alive: 1000,
    };

    match thread::Builder::new()
        .name("Client-Communicator".into())
        .spawn(move || send_packets_to_client(client_sender_2, client_receiver, stream_cloned, id))
    {
        Ok(_) => {}
        Err(_) => {
            error!("Error running client communicator");
        }
    }

    read_packets_from_client(&mut current_client, password_required)
}

fn read_packets_from_client(current_client: &mut ClientFlags, password_required: bool) {
    loop {
        let mut num_buffer = [0u8; 1];
        match current_client.connection.read_exact(&mut num_buffer) {
            Ok(_) => {
                let packet_type = num_buffer[0].into();
                match remaining_length_read(&mut current_client.connection) {
                    Ok(buff_size) => {
                        match read_packet(
                            current_client,
                            packet_type,
                            buff_size,
                            num_buffer[0],
                            password_required,
                        ) {
                            Ok(_) => {}
                            Err(_) => {
                                error!("Error trying to read a packet");
                            }
                        }
                    }
                    Err(_) => {
                        error!("Error trying to read buffer size");
                    }
                }
            }
            Err(_) => {
                inform_client_disconnect_to_coordinator(
                    current_client,
                    Vec::new(),
                    Packet::Disgrace,
                );
                info!("Client disconnected disgracefully");
                break;
            }
        }
    }
}
/// Receive messages from the coordinator and send messages to client
///
fn send_packets_to_client(
    client_sender: Arc<Mutex<Sender<PacketThings>>>,
    client_receiver: Receiver<Vec<u8>>,
    mut stream_cloned: TcpStream,
    thread_id: usize,
) {
    loop {
        match client_receiver.recv() {
            Ok(val) => {
                if val[0] == 255 {
                    info!("Closing Client-Communicator thread");
                    break;
                }
                match stream_cloned.write_all(&val) {
                    Ok(_) => {}
                    Err(_) => {
                        let sender = client_sender.lock();
                        match sender {
                            Ok(sender_ok) => {
                                let packet_to_server = PacketThings {
                                    thread_id,
                                    packet_type: Packet::Disgrace,
                                    bytes: Vec::new(),
                                };
                                match sender_ok.send(packet_to_server) {
                                    Ok(_) => {
                                        info!(
                                            "Success sending disgraceful connection to Coordinator."
                                        );
                                        break;
                                    }
                                    Err(_) => {
                                        warn!(
                                            "Error sending disgraceful connection to Coordinator."
                                        )
                                    }
                                };
                            }
                            Err(_) => {
                                warn!("Error reading coordinator channel.")
                            }
                        }
                    }
                }
            }
            Err(_er) => {}
        }
    }
}
