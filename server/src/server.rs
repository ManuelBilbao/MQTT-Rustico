use crate::client::Client;
use crate::configuration::Configuration;
use crate::coordinator::run_coordinator;
use crate::packet::{inform_client_disconnect_to_coordinator, read_packet, Packet};
use crate::utils::remaining_length_read;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use tracing::{debug, info, warn};

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
        let _aux = config.set_config(file_path); //Manejar
        Server { cfg: config }
    }

    pub fn run(&self) -> std::io::Result<()> {
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
            let client_sender_1 = Arc::clone(clients_sender);
            let client_sender_2 = Arc::clone(clients_sender);
            let client: Client = Client::new(index, coordinator_sender);
            handler_clients_lock
                .lock()
                .unwrap()
                .insert(client.thread_id, client);
            thread::Builder::new()
                .name("Client-Listener".into())
                .spawn(move || {
                    info!("Se lanzo un nuevo cliente.");
                    handle_client(
                        index,
                        &mut client_stream,
                        client_sender_1,
                        client_sender_2,
                        client_receiver,
                    );
                })
                .unwrap();
            index += 1;
        }
    }
}

pub fn handle_client(
    id: usize,
    stream: &mut TcpStream,
    client_sender_1: Arc<Mutex<Sender<PacketThings>>>,
    client_sender_2: Arc<Mutex<Sender<PacketThings>>>,
    client_receiver: Receiver<Vec<u8>>,
) {
    let stream_cloned = stream.try_clone().unwrap();
    let mut current_client = ClientFlags {
        id,
        client_id: None,
        connection: stream,
        sender: client_sender_1,
        clean_session: 1,
        keep_alive: 1000,
    };

    thread::Builder::new()
        .name("Client-Communicator".into())
        .spawn(move || send_packets_to_client(client_sender_2, client_receiver, stream_cloned, id))
        .unwrap();

    read_packets_from_client(&mut current_client)
}

fn read_packets_from_client(mut current_client: &mut ClientFlags) {
    loop {
        let mut num_buffer = [0u8; 1]; //Recibimos 2 bytes
        match current_client.connection.read_exact(&mut num_buffer) {
            Ok(_) => {
                //Acordarse de leerlo  como BE, let mensaje = u32::from_be_bytes(num_buffer);
                let packet_type = num_buffer[0].into();
                let buff_size = remaining_length_read(&mut current_client.connection).unwrap();
                read_packet(&mut current_client, packet_type, buff_size, num_buffer[0]).unwrap();
            }
            Err(_) => {
                inform_client_disconnect_to_coordinator(
                    current_client,
                    Vec::new(),
                    Packet::Disgrace,
                );
                info!("El cliente se desconecto disgracefully");
                break;
            }
        }
    }
}

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
                    info!("Cerrando thread de comunicación con el cliente");
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
                                            "Success sending disgraceful connection to coordinator"
                                        )
                                    }
                                    Err(_) => {
                                        debug!(
                                            "Error sending disgraceful connection to coordinator"
                                        )
                                    }
                                };
                            }
                            Err(_) => {
                                warn!("Error reading coordinator channel")
                            }
                        }
                    }
                }
            }
            Err(_er) => {}
        }
    }
}
