use crate::client::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use tracing::{info, warn};
use std::thread;

pub fn run_stacked_coordinator(lock_clients: Arc<Mutex<HashMap<usize, Client>>>) {
    loop {
        thread::sleep(Duration::from_secs(1));
        match lock_clients.lock() {
            Ok(mut locked) => {
                for client in locked.iter_mut() {
                    if !client.1.disconnected {
                        for message in client.1.publishes_received.iter() {
                            match client.1.channel.send(message.clone()) {
                                Ok(_) => info!("Publish sended succesfully"),
                                Err(_) => warn!("Error sending publish to client"),
                            }
                        }
                    }
                }
            }
            Err(_) => {
                warn!("Unable to get the clients lock");
            }
        }
    }
}
