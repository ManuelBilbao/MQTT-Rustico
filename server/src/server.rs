use crate::cliente::Client;
use crate::configuracion::Configuracion;
use crate::paquete::{
    leer_paquete, verificar_nombre_protocolo, verificar_version_protocolo, Paquetes,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

const CONEXION_IDENTIFICADOR_RECHAZADO: u8 = 2;
const _CONEXION_PROTOCOLO_RECHAZADO: u8 = 1;
const CONEXION_SERVIDOR_INCORRECTO: u8 = 1;
const _CONEXION_EXITOSA: u8 = 0;

pub struct Server {
    //
    cfg: Configuracion,
    _clientes: Vec<Client>, //
}

pub struct FlagsCliente<'a> {
    pub id: usize,
    client_id: Option<String>,
    pub conexion: &'a mut TcpStream,
    pub sender: Arc<Mutex<Sender<Paquete>>>,
    username: Option<String>,
    password: Option<String>,
    will_topic: Option<String>,
    will_message: Option<String>,
    will_qos: u8,
    will_retained: bool,
    keep_alive: u16,
}

pub struct Paquete {
    pub thread_id: usize,
    pub packet_type: Paquetes,
    pub bytes: Vec<u8>,
}

impl Server {
    pub fn new(file_path: &str) -> Self {
        let mut config = Configuracion::new();
        let _aux = config.set_config(file_path); //Manejar
        Server {
            cfg: config,
            _clientes: Vec::new(),
        }
    }

    pub fn run(&self) -> std::io::Result<()> {
        let address = self.cfg.get_address();
        println!("IP: {}", &address);
        let clientes: Vec<Client> = Vec::new();
        let lock_clientes = Arc::new(Mutex::new(clientes));
        let lock_clientes_para_handler = lock_clientes.clone();
        let (sender_de_los_clientes, receiver_del_coordinador): (
            Sender<Paquete>,
            Receiver<Paquete>,
        ) = mpsc::channel();
        let sender_de_los_clientes = Arc::new(Mutex::new(sender_de_los_clientes));
        thread::Builder::new()
            .name("Coordinator".into())
            .spawn(move || loop {
                match receiver_del_coordinador.recv() {
                    Ok(paquete) => {
                        match paquete.packet_type {
                            Paquetes::Subscribe => {
                                let vector_con_qos =
                                    Server::procesar_subscribe(&lock_clientes, &paquete);
                                Server::enviar_subback(&lock_clientes, paquete, vector_con_qos)
                            }
                            Paquetes::Unsubscribe => {
                                Server::procesar_unsubscribe(&lock_clientes, &paquete);
                                Server::enviar_unsubback(&lock_clientes, paquete)
                            }
                            _ => {}
                        }
                        if lock_clientes.lock().unwrap().is_empty() {
                            println!("Esta vacio, no lo ves?");
                        }
                    }
                    Err(_e) => {
                        println!("error del coordinador al recibir un paquete");
                    }
                }
            })?;

        // Thread Listener
        Server::wait_new_clients(
            &address,
            lock_clientes_para_handler,
            &sender_de_los_clientes,
        )
    }

    fn enviar_unsubback(lock_clientes: &Arc<Mutex<Vec<Client>>>, paquete: Paquete) {
        let buffer: Vec<u8> = vec![Paquetes::UnsubAck.into(), 0x02, paquete.bytes[0], paquete.bytes[1]];
        match lock_clientes.lock() {
            Ok(locked) => {
                if let Some(indice) = locked.iter().position(|r| r.id == paquete.thread_id) {
                    match locked[indice].channel.send(buffer) {
                        Ok(_) => {
                            println!("SubBack enviado")
                        }
                        Err(_) => {
                            println!("Error al enviar Subback")
                        }
                    }
                }
            }
            Err(_) => {
                println!("Imposible acceder al lock desde el cordinador")
            }
        }
    }

    fn procesar_unsubscribe(lock_clientes: &Arc<Mutex<Vec<Client>>>, paquete: &Paquete) {
        let mut indice = 2;
        while indice < paquete.bytes.len() {
            let tamanio_topic: usize =
                ((paquete.bytes[indice] as usize) << 8) + paquete.bytes[indice + 1] as usize;
            indice += 2;
            let topico = bytes2string(&paquete.bytes[indice..(indice + tamanio_topic)]).unwrap();
            indice += tamanio_topic;
            match lock_clientes.lock() {
                Ok(mut locked) => {
                    if let Some(indice) = locked.iter().position(|r| r.id == paquete.thread_id) {
                        locked[indice].unsubscribe(topico);
                    }
                }
                Err(_) => {
                    println!("Error al intentar desuscribir de un topico")
                }
            }
        }
    }

    fn enviar_subback(
        lock_clientes: &Arc<Mutex<Vec<Client>>>,
        paquete: Paquete,
        vector_con_qos: Vec<u8>,
    ) {
        let mut buffer: Vec<u8> = vec![
            Paquetes::SubAck.into(),
            (vector_con_qos.len() as u8 + 2_u8) as u8,
            paquete.bytes[0],
            paquete.bytes[1],
        ];
        for bytes in vector_con_qos {
            buffer.push(bytes);
        }
        match lock_clientes.lock() {
            Ok(locked) => {
                if let Some(indice) = locked.iter().position(|r| r.id == paquete.thread_id) {
                    match locked[indice].channel.send(buffer) {
                        Ok(_) => {
                            println!("SubBack enviado")
                        }
                        Err(_) => {
                            println!("Error al enviar Subback")
                        }
                    }
                }
            }
            Err(_) => {
                println!("Imposible acceder al lock desde el cordinador")
            }
        }
    }

    fn procesar_subscribe(lock_clientes: &Arc<Mutex<Vec<Client>>>, paquete: &Paquete) -> Vec<u8> {
        let mut indice = 2;
        let mut vector_con_qos: Vec<u8> = Vec::new();
        while indice < paquete.bytes.len() {
            let tamanio_topic: usize =
                ((paquete.bytes[indice] as usize) << 8) + paquete.bytes[indice + 1] as usize;
            indice += 2;
            let topico = bytes2string(&paquete.bytes[indice..(indice + tamanio_topic)]).unwrap();
            indice += tamanio_topic;
            let qos: u8 = &paquete.bytes[indice] & 0x01;
            match lock_clientes.lock() {
                Ok(mut locked) => {
                    if let Some(indice) = locked.iter().position(|r| r.id == paquete.thread_id) {
                        locked[indice].subscribe(topico);
                        vector_con_qos.push(qos);
                    }
                }
                Err(_) => vector_con_qos.push(0x80),
            }
        }
        vector_con_qos
    }

    fn wait_new_clients(
        address: &str,
        lock_clientes_para_handler: Arc<Mutex<Vec<Client>>>,
        sender_de_los_clientes: &Arc<Mutex<Sender<Paquete>>>,
    ) -> std::io::Result<()> {
        let mut indice: usize = 0;
        loop {
            let listener = TcpListener::bind(&address)?;
            let connection = listener.accept()?;
            let mut client_stream: TcpStream = connection.0;
            let (sender_del_coordinador, receiver_del_cliente): (
                Sender<Vec<u8>>,
                Receiver<Vec<u8>>,
            ) = mpsc::channel();
            let sender_del_cliente = Arc::clone(sender_de_los_clientes);
            let cliente: Client = Client::new(indice, sender_del_coordinador);
            lock_clientes_para_handler.lock().unwrap().push(cliente);
            thread::Builder::new()
                .name("Client-Listener".into())
                .spawn(move || {
                    handle_client(
                        indice,
                        &mut client_stream,
                        sender_del_cliente,
                        receiver_del_cliente,
                    );
                })
                .unwrap();
            indice += 1;
        }
    }
}

///////////////////////a partir de aca puede ser que lo movamos o renombremos (para mi va aca pero renombrado)

pub fn handle_client(
    id: usize,
    stream: &mut TcpStream,
    sender_del_cliente: Arc<Mutex<Sender<Paquete>>>,
    receiver_del_cliente: Receiver<Vec<u8>>,
) {
    let mut stream_clonado = stream.try_clone().unwrap();
    let mut cliente_actual = FlagsCliente {
        id,
        client_id: None,
        conexion: stream,
        sender: sender_del_cliente,
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
            match receiver_del_cliente.recv() {
                Ok(val) => {
                    stream_clonado.write_all(&val).unwrap();
                }
                Err(_er) => {}
            }
        })
        .unwrap();

    loop {
        let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
        match cliente_actual.conexion.read_exact(&mut num_buffer) {
            Ok(_) => {
                //Acordarse de leerlo  como BE, let mensaje = u32::from_be_bytes(num_buffer);
                let tipo_paquete = num_buffer[0].into();
                leer_paquete(&mut cliente_actual, tipo_paquete, num_buffer[1]).unwrap();
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
        Err(_) => Err(CONEXION_SERVIDOR_INCORRECTO),
    }
}

pub fn realizar_conexion(cliente: &mut FlagsCliente, buffer_paquete: Vec<u8>) -> Result<u8, u8> {
    verificar_nombre_protocolo(&buffer_paquete)?;
    verificar_version_protocolo(&buffer_paquete[6])?;

    let flag_username = buffer_paquete[7] & 0x80 == 0x80;
    let flag_password = buffer_paquete[7] & 0x40 == 0x40;
    let flag_will_retain = buffer_paquete[7] & 0x20 == 0x20;
    let flag_will_qos = (buffer_paquete[7] & 0x18) >> 3;
    let flag_will_flag = buffer_paquete[7] & 0x04 == 0x04;
    let flag_clean_session = buffer_paquete[7] & 0x02 == 0x02;

    let keep_alive: u16 = ((buffer_paquete[8] as u16) << 8) + buffer_paquete[9] as u16;

    let tamanio_client_id: usize =
        ((buffer_paquete[10] as usize) << 8) + buffer_paquete[11] as usize;

    let client_id = Some(bytes2string(&buffer_paquete[12..12 + tamanio_client_id])?); // En UTF-8

    let mut indice: usize = (12 + tamanio_client_id) as usize;

    // Atajar si tamanio_x = 0
    let mut will_topic = None;
    let mut will_message = None;
    if flag_will_flag {
        let tamanio_will_topic: usize =
            ((buffer_paquete[indice] as usize) << 8) + buffer_paquete[(indice + 1)] as usize;
        indice += 2_usize;
        will_topic = Some(bytes2string(
            &buffer_paquete[indice..(indice + tamanio_will_topic)],
        )?);
        indice += tamanio_will_topic;

        let tamanio_will_message: usize = ((buffer_paquete[indice] as usize) << 8)
            + buffer_paquete[(indice + 1) as usize] as usize;
        indice += 2_usize;
        will_message = Some(bytes2string(
            &buffer_paquete[indice..(indice + tamanio_will_message)],
        )?);
        indice += tamanio_will_message;
    }

    let mut username: Option<String> = None;
    if flag_username {
        let tamanio_username: usize = ((buffer_paquete[indice] as usize) << 8)
            + buffer_paquete[(indice + 1) as usize] as usize;
        indice += 2_usize;
        username = Some(bytes2string(
            &buffer_paquete[indice..(indice + tamanio_username)],
        )?);
        indice += tamanio_username;
    }

    let mut password: Option<String> = None;
    if flag_password {
        let tamanio_password: usize = ((buffer_paquete[indice] as usize) << 8)
            + buffer_paquete[(indice + 1) as usize] as usize;
        indice += 2_usize;
        password = Some(bytes2string(
            &buffer_paquete[indice..(indice + tamanio_password)],
        )?);
    }

    //PROCESAR
    if client_id == None && !flag_clean_session {
        return Err(CONEXION_IDENTIFICADOR_RECHAZADO);
    }

    if flag_will_retain && flag_will_qos == 2 {
        //
    }

    cliente.client_id = client_id;
    cliente.username = username;
    cliente.password = password;
    cliente.will_topic = will_topic;
    cliente.will_message = will_message;
    cliente.will_qos = flag_will_qos;
    cliente.will_retained = flag_will_retain;
    cliente.keep_alive = keep_alive;

    Ok(1) // TODO: Persistent Sessions
}
