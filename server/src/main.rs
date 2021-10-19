use std::env::args;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io::Write;
use std::io::{Read};
use std::sync::{mpsc, Mutex, Arc};
use std::sync::mpsc::{Sender, Receiver};
use crate::configuracion::Configuracion;
use crate::paquete::{verificar_nombre_protocolo, verificar_version_protocolo, leer_paquete, get_tipo};

mod configuracion;
mod paquete;

const CONEXION_IDENTIFICADOR_RECHAZADO: u8 = 2;
const CONEXION_PROTOCOLO_RECHAZADO: u8 = 1;
const CONEXION_SERVIDOR_INCORRECTO: u8 = 1;
const CONEXION_EXITOSA: u8 = 0;
static SERVER_ARGS: usize = 2;

pub struct Cliente<'a> {
    id: Option<String>,
    conexion: &'a mut TcpStream,
    sender: Arc<Mutex<Sender<String>>>,
    username: Option<String>,
    password: Option<String>,
    will_topic: Option<String>,
    will_message: Option<String>,
    will_qos: u8,
    will_retained: bool,
    keep_alive: u16
}

fn main() -> Result<(), ()> {
    let argv = args().collect::<Vec<String>>();
    if argv.len() != SERVER_ARGS {
        println!("Cantidad de argumentos inválido");
        return Err(());
    }
    let mut config = Configuracion::new();
    let aux = config.set_config(&argv[1]); //Manejar
    let address = config.get_address();
    println!("Address = {}", &address);
    server_run(&address).unwrap();
    Ok(())
}

fn server_run(address: &str) -> std::io::Result<()> {
    let (server_sender, cliente_receiver) : (Sender<String>, Receiver<String>) = mpsc::channel();
    let (cliente_sender, server_receiver) : (Sender<String>, Receiver<String>) = mpsc::channel();
    let cliente_receiver = Arc::new(Mutex::new(cliente_receiver));
    let cliente_sender = Arc::new(Mutex::new(cliente_sender));
    thread::spawn( move || {
        loop{
            let leido = server_receiver.recv().unwrap();
            server_sender.send(leido).expect("Error en el sender");
        }
    });
    loop {
        let listener = TcpListener::bind(address)?;
        let connection = listener.accept()?;
        let mut client_stream : TcpStream = connection.0;
        let nuevo_receiver = Arc::clone(&cliente_receiver);
        let nuevo_sender = Arc::clone(&cliente_sender);
        thread::spawn(move || {
            handle_client (&mut client_stream,nuevo_receiver, nuevo_sender);
        });
    }
}

fn handle_client(stream: &mut TcpStream, receiver: Arc<Mutex<Receiver<String>>>, sender: Arc<Mutex<Sender<String>>>) {
    let mut stream_clonado = stream.try_clone().unwrap();
    let mut cliente_actual = Cliente{
        id: None,
        conexion: &mut stream_clonado,
        sender: sender,
        username: None,
        password: None,
        will_topic: None,
        will_message: None,
        will_qos: 0,
        will_retained: false,
        keep_alive: 1000
    };
    /*let lock = Arc::new(Mutex::new(&cliente_actual));
    let lock_para_thread = Arc::clone(&lock); //lock.clone();
    thread::spawn( move || {
        loop{
            let leido = receiver.lock().unwrap().recv().unwrap();
            let mut estructura = lock_para_thread.lock().unwrap();
            //LEER SUSCRIPCIONES
        }
    });*/
    loop {
        let mut num_buffer = [0u8; 2]; //Recibimos 2 bytes
        match stream.read_exact(&mut num_buffer) {
            Ok(_) => {
                //Acordarse de leerlo  como BE, let mensaje = u32::from_be_bytes(num_buffer);
                let tipo_paquete = get_tipo(num_buffer[0]);
                leer_paquete(&mut cliente_actual, tipo_paquete, num_buffer[1]).unwrap();
            },
            Err(_) => {
            }
        }
    }
}

fn realizar_publicacion(buffer_paquete: Vec<u8>) -> Result<(), String> {
    Err("error".to_owned())
}

fn bytes2string(bytes: &[u8]) -> Result<String, u8> {
    match std::str::from_utf8(bytes) {
        Ok(str) => Ok(str.to_owned()),
        Err(_) => Err(CONEXION_SERVIDOR_INCORRECTO)
    }
}

fn realizar_conexion(cliente : &mut Cliente, buffer_paquete: Vec<u8>) -> Result<u8, u8> {
    verificar_nombre_protocolo(&buffer_paquete)?;
    verificar_version_protocolo(&buffer_paquete[6])?;

    let flag_username = buffer_paquete[7] & 0x80 == 0x80;
    let flag_password = buffer_paquete[7] & 0x40 == 0x40;
    let flag_will_retain = buffer_paquete[7] & 0x20 == 0x20;
    let flag_will_qos = (buffer_paquete[7] & 0x18) >> 3;
    let flag_will_flag = buffer_paquete[7] & 0x04 == 0x04;
    let flag_clean_session = buffer_paquete[7] & 0x02 == 0x02;

    let keep_alive :u16 = ((buffer_paquete[8] as u16) << 8) + buffer_paquete[9] as u16;

    let tamanio_client_id:usize = ((buffer_paquete[10] as usize) << 8) + buffer_paquete[11] as usize;

    let client_id = Some(bytes2string(&buffer_paquete[12..12+tamanio_client_id])?); // En UTF-8

    let mut indice: usize = (12+tamanio_client_id) as usize;

    // Atajar si tamanio_x = 0
    let mut will_topic = None;
    let mut will_message = None;
    if flag_will_flag {
        let tamanio_will_topic: usize = ((buffer_paquete[indice] as usize) << 8) + buffer_paquete[(indice+1)] as usize;
        indice += 2 as usize;
        will_topic = Some(bytes2string(&buffer_paquete[indice..(indice+tamanio_will_topic)])?);
        indice += tamanio_will_topic;

        let tamanio_will_message: usize = ((buffer_paquete[indice] as usize) << 8) + buffer_paquete[(indice+1) as usize] as usize;
        indice += 2 as usize;
        will_message = Some(bytes2string(&buffer_paquete[indice..(indice+tamanio_will_message)])?);
        indice += tamanio_will_message;
    }

    let mut username : Option<String> = None;
    if flag_username {
        let tamanio_username: usize = ((buffer_paquete[indice] as usize) << 8) + buffer_paquete[(indice+1) as usize] as usize;
        indice += 2 as usize;
        username = Some(bytes2string(&buffer_paquete[indice..(indice+tamanio_username)])?);
        indice += tamanio_username;
    }

    let mut password : Option<String> = None;
    if flag_password {
        let tamanio_password:usize = ((buffer_paquete[indice] as usize) << 8) + buffer_paquete[(indice+1) as usize] as usize;
        indice += 2 as usize;
        password = Some(bytes2string(&buffer_paquete[indice..(indice+tamanio_password)])?);
    }

    //PROCESAR
    if client_id == None && !flag_clean_session{
        return Err(CONEXION_IDENTIFICADOR_RECHAZADO);
    }

    if flag_will_retain && flag_will_qos == 2{
        //
    }

    cliente.id = client_id;
    cliente.username = username;
    cliente.password = password;
    cliente.will_topic = will_topic;
    cliente.will_message = will_message;
    cliente.will_qos = flag_will_qos;
    cliente.will_retained = flag_will_retain;
    cliente.keep_alive = keep_alive;

    Ok(1) // TODO: Persistent Sessions
}
