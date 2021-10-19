use std::env::args;
use crate::server::Server;

mod configuracion;
mod paquete;
mod cliente;
mod server;

static SERVER_ARGS: usize = 2;

fn main() -> Result<(), ()> {
    let argv = args().collect::<Vec<String>>();
    if argv.len() != SERVER_ARGS {
        println!("Cantidad de argumentos inválido");
        return Err(());
    }
    let mut server = Server::new(&argv[1]);
    server.run().unwrap();
    Ok(())
}
