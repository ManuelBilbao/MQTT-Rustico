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
}
