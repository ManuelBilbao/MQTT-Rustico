use crate::wildcard::compare_topic;
use std::sync::mpsc::Sender;

pub struct Client {
    pub id: usize,
    pub channel: Sender<Vec<u8>>,
    topics: Vec<String>,
}

impl Client {
    pub fn new(id: usize, channel: Sender<Vec<u8>>) -> Self {
        Client {
            id,
            channel,
            topics: Vec::new(),
        }
    }

    pub fn subscribe(&mut self, topic: String) {
        self.topics.push(topic);
    }

    pub fn unsubscribe(&mut self, topic: String) {
        if let Some(indice) = self.topics.iter().position(|r| *r == topic) {
            self.topics.remove(indice);
        }
    }

    pub fn is_subscribed_to(&self, topic: &str) -> bool {
        let mut subscribed: bool = false;
        for topic_aux in &self.topics {
            if compare_topic(topic, topic_aux) {
                subscribed = true;
            }
        }
        subscribed
    }
}