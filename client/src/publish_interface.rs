extern crate glib;
extern crate gtk;
use self::gtk::atk::glib::clone;
use crate::packet::send_publish_packet;
use gtk::prelude::*;
use std::io;
use std::net::TcpStream;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;

pub fn build_publish_ui(
    stream: &mut TcpStream,
    puback_receiver: Receiver<String>,
    connect_builder: &gtk::Builder,
) {
    let mut pub_obj = ReceiverObject::new().expect("Error when creating ReceiverObject");
    let (pub_sender, pub_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let message_entry: gtk::Entry = connect_builder
        .object("message_entry")
        .expect("Error when getting object from glade");
    let topic_entry: gtk::Entry = connect_builder
        .object("topic_entry")
        .expect("Error when getting object from glade");
    let publish_button: gtk::Button = connect_builder
        .object("publish_button")
        .expect("Error when getting object from glade");
    let qos_publish_switch: gtk::Switch = connect_builder
        .object("QOS_publish_switch")
        .expect("Error when getting object from glade");
    let retained_check: gtk::ToggleButton = connect_builder
        .object("retained_check")
        .expect("Error when getting object from glade");
    let stream_clone = stream.try_clone().expect("Cannot clone stream");

    publish_button.connect_clicked(clone!(@weak message_entry, @weak topic_entry, @weak qos_publish_switch, @weak retained_check => move |_|{
        let message = message_entry.text();
        let topic = topic_entry.text();
        let mut stream_clone2 = stream_clone.try_clone().expect("Cannot clone stream");
        send_publish_packet(&mut stream_clone2, topic.to_string(), message.to_string(), false, qos_publish_switch.is_active(), retained_check.is_active());
        message_entry.set_properties(&[("text", &"".to_owned())]).expect("Error when setting properties");
        topic_entry.set_properties(&[("text", &"".to_owned())]).expect("Error when setting properties");
    }));
    pub_obj.build(connect_builder, pub_receiver, "puback_label");
    pub_obj.start(pub_sender, puback_receiver);
}
/// Structure that is used to receive text from the client and set it to a label
pub struct ReceiverObject {}

impl ReceiverObject {
    pub fn new() -> io::Result<Self> {
        Ok(Self {})
    }
    ///When the glib_receiver receives text, the label's text is updated.
    pub fn build(&self, builder: &gtk::Builder, glib_receiver: glib::Receiver<String>, name: &str) {
        match builder.object::<gtk::Label>(name) {
            Some(label) => {
                glib_receiver.attach(None, move |text: String| {
                    let text_to_be_set;
                    if text != "Publish sent successfully\n" && text != "Connected successfully\n" {
                        text_to_be_set = label.text().to_string() + "\n" + &text;
                    } else {
                        text_to_be_set = text;
                    }
                    label.set_text(text_to_be_set.as_str());
                    glib::Continue(true)
                });
            }
            None => {
                println!("Error when getting label from glade");
            }
        }
    }
/// This is used for the topic_updater. When the glib_receiver receives text, the label's text is updated.
    pub fn update_subs_upon_publish(
        &self,
        glib_receiver: glib::Receiver<String>,
        label_lock: Arc<Mutex<gtk::Label>>,
    ) {
        glib_receiver.attach(None, move |text: String| {
            let topic_compare = "\n".to_string() + &text + "\n";
            match label_lock.lock() {
                Ok(label) => {
                    if label.text().matches(&topic_compare).count() == 0 {
                        let text = label.text().to_string() + &text + "\n";
                        label.set_text(text.as_str());
                    }
                }
                Err(_) => {
                    println!("Error when updating current subscriptions");
                }
            }
            glib::Continue(true)
        });
    }
/// Spawns a thread that will listen for text sent by the client and then send it through the glib channel. This is where the channel receiver created in "run_client_and_build_windows" is used.
    pub fn start(&mut self, glib_sender: glib::Sender<String>, receiver: Receiver<String>) {
        thread::spawn(move || loop {
            match receiver.recv() {
                Ok(text) => {
                    glib_sender
                        .send(text)
                        .expect("Couldn't send data to channel");
                }
                Err(_) => {
                    println!("Error when receiving data");
                }
            }
        });
    }
}
