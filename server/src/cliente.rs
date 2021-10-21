use std::sync::mpsc::Sender;

pub struct Client {
    _id: usize,
    _channel: Sender<Vec<u8>>,
    _topics: Vec<String>,
}

impl Client {
    pub fn new(_id: usize, _channel: Sender<Vec<u8>>) -> Self {
        Client {
            _id,
            _channel,
            _topics: Vec::new(),
        }
    }

    pub fn _suscribe(&mut self, topic: String) {
        self._topics.push(topic);
    }

    pub fn _unsuscribe(&mut self, topic: String) {
        if let Some(indice) = self._topics.iter().position(|r| *r == topic) {
            self._topics.remove(indice);
        }
    }
}
