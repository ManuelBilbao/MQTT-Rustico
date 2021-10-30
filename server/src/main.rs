use crate::server::Server;
use std::env::args;

mod client;
mod configuration;
mod coordinator;
mod packet;
mod server;

static SERVER_ARGS: usize = 2;

fn main() -> Result<(), ()> {
    let argv = args().collect::<Vec<String>>();
    if argv.len() != SERVER_ARGS {
        println!("Invalid number of arguments");
        return Err(());
    }
    let server = Server::new(&argv[1]);
    server.run().unwrap();
    Ok(())
}
