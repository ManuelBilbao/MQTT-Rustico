use crate::server::Server;
use std::env::args;

mod cliente;
mod configuracion;
mod coordinator;
mod paquete;
mod server;

static SERVER_ARGS: usize = 2;

fn main() -> Result<(), ()> {
    let argv = args().collect::<Vec<String>>();
    if argv.len() != SERVER_ARGS {
        println!("Cantidad de argumentos inválido");
        return Err(());
    }
    let server = Server::new(&argv[1]);
    server.run().unwrap();
    Ok(())
}
