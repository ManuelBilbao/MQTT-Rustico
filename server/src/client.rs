//! # Client
//!
//! Useful structures to handle clients.

use crate::wildcard::compare_topic;
use std::sync::mpsc::Sender;

/// A structure to save subscription topics with QoS level.
pub struct Subscription {
    pub topic: String,
    pub qos: u8,
}

/// A structure containing the client's info that the server needs, such as:
/// - `channel` to send packets to the client.
/// - `topics` to which the client is subscribed.
/// - `publishes_received`, storing all packets that need to be sent to the client.
pub struct Client {
    pub thread_id: usize,
    pub client_id: String,
    pub channel: Sender<Vec<u8>>,
    pub topics: Vec<Subscription>,
    pub publishes_received: Vec<Vec<u8>>,
    pub clean_session: u8,
    pub lastwill_topic: Option<String>,
    pub lastwill_message: Option<String>,
    pub lastwill_qos: u8,
    pub lastwill_retained: bool,
    pub disconnected: bool,
}

impl Client {
    /// Creates a new Client.
    pub fn new(thread_id: usize, channel: Sender<Vec<u8>>) -> Self {
        Client {
            thread_id,
            client_id: "".to_owned(),
            channel,
            topics: Vec::new(),
            publishes_received: Vec::new(),
            clean_session: 0,
            lastwill_topic: None,
            lastwill_message: None,
            lastwill_qos: 0,
            lastwill_retained: false,
            disconnected: true,
        }
    }

    /// Subscribe the client to `topic` with `qos` level.
    pub fn subscribe(&mut self, topic: String, qos: u8) {
        let new_subscription = Subscription { topic, qos };
        self.topics.push(new_subscription);
    }

    /// Unsubscribe the client from `topic`.
    pub fn unsubscribe(&mut self, topic: String) {
        if let Some(indice) = self.topics.iter().position(|r| *(r.topic) == topic) {
            self.topics.remove(indice);
        }
    }

    /// Whether client is subscribed to `topic` with QoS 1 or not.
    pub fn is_subscribed_to_qos1(&self, topic: &str) -> bool {
        let mut subscribed: bool = false;
        for topic_aux in &self.topics {
            if topic_aux.qos == 1 && compare_topic(topic, topic_aux.topic.as_str()) {
                subscribed = true;
            }
        }
        subscribed
    }

    /// Whether the client is subscribed to `topic` or not.
    pub fn is_subscribed_to(&self, topic: &str) -> bool {
        let mut subscribed: bool = false;
        for topic_aux in &self.topics {
            if compare_topic(topic, topic_aux.topic.as_str()) {
                subscribed = true;
            }
        }
        subscribed
    }

    /// Clear topic subscriptions list and publishes queue.
    pub fn remove_subscriptions_and_queue(&mut self) {
        self.topics = Vec::new();
        self.publishes_received = Vec::new();
    }
}
