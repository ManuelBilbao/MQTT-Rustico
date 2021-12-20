//! # Stacked Messages
//!
//! Thread to send messages to client (specifically QoS 1 publish messages) until it returns the _Puback_ packet.

use crate::client::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

/// Send queued publish messages to client periodically.
pub fn run_stacked_coordinator(lock_clients: Arc<Mutex<HashMap<usize, Client>>>) {
    loop {
        thread::sleep(Duration::from_secs(1));
        match lock_clients.lock() {
            Ok(mut locked) => {
                for (_, client) in locked.iter_mut() {
                    if !client.disconnected {
                        for message in client.publishes_received.iter() {
                            match client.channel.send(message.clone()) {
                                Ok(_) => {
                                    info!("Publish sent successfully.");
                                }
                                Err(_) => warn!("Error sending publish to client."),
                            }
                        }
                        let mut i: usize = 0;
                        while i < client.publishes_received.len() {
                            let byte_0 = client.publishes_received[i as usize][0];
                            if (byte_0 & 0x02) == 0 {
                                client.publishes_received.remove(i as usize);
                            } else {
                                i += 1;
                            }
                        }
                    }
                }
            }
            Err(_) => {
                warn!("Unable to get the clients lock.");
            }
        }
    }
}
