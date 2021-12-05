use crate::wildcard::compare_topic;
use std::sync::mpsc::Sender;

pub struct Subscription {
    pub topic: String,
    pub qos: u8,
}

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

    pub fn subscribe(&mut self, topic: String, qos: u8) {
        let new_subscription = Subscription { topic, qos };
        self.topics.push(new_subscription);
    }

    pub fn unsubscribe(&mut self, topic: String) {
        if let Some(indice) = self.topics.iter().position(|r| *(r.topic) == topic) {
            self.topics.remove(indice);
        }
    }

    pub fn is_subscribed_to_qos1(&self, topic: &str) -> bool {
        let mut subscribed: bool = false;
        for topic_aux in &self.topics {
            if topic_aux.qos == 1 && compare_topic(topic, topic_aux.topic.as_str()) {
                subscribed = true;
            }
        }
        subscribed
    }

    pub fn is_subscribed_to(&self, topic: &str) -> bool {
        let mut subscribed: bool = false;
        for topic_aux in &self.topics {
            if compare_topic(topic, topic_aux.topic.as_str()) {
                subscribed = true;
            }
        }
        subscribed
    }
}
