extern crate glib;
extern crate gtk;
use self::gtk::atk::glib::clone;
use crate::packet::send_publish_packet;
use gtk::prelude::*;
use std::io;
use std::net::TcpStream;
use std::sync::mpsc::Receiver;
use std::thread;

pub fn build_publish_ui(
    stream: &mut TcpStream,
    _client_id: String,
    puback_receiver: Receiver<String>,
    connect_builder: &gtk::Builder,
) {
    let mut pub_window = ReceiverWindow::new().unwrap();
    let (pub_sender, pub_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let message_entry: gtk::Entry = connect_builder.object("message_entry").unwrap();
    let topic_entry: gtk::Entry = connect_builder.object("topic_entry").unwrap();
    let publish_button: gtk::Button = connect_builder.object("publish_button").unwrap();
    let qos_publish_switch: gtk::Switch = connect_builder.object("QOS_publish_switch").unwrap();
    let retained_check: gtk::ToggleButton = connect_builder.object("retained_check").unwrap();
    let stream_clone = stream.try_clone().expect("Cannot clone stream");

    publish_button.connect_clicked(clone!(@weak message_entry, @weak topic_entry, @weak qos_publish_switch, @weak retained_check => move |_|{
        let message = message_entry.text();
        let topic = topic_entry.text();
        let mut stream_clone2 = stream_clone.try_clone().expect("Cannot clone stream");
        send_publish_packet(&mut stream_clone2, topic.to_string(), message.to_string(), false, qos_publish_switch.is_active(), retained_check.is_active());
        message_entry.set_properties(&[("text", &"".to_owned())]).unwrap();
        topic_entry.set_properties(&[("text", &"".to_owned())]).unwrap();
    }));
    pub_window.build(connect_builder, pub_receiver, "puback_label");
    pub_window.start(pub_sender, puback_receiver);
}

pub struct ReceiverWindow {}

impl ReceiverWindow {
    pub fn new() -> io::Result<Self> {
        Ok(Self {})
    }
    pub fn build(&self, builder: &gtk::Builder, glib_receiver: glib::Receiver<String>, name: &str) {
        let label: gtk::Label = builder.object(name).unwrap();

        glib_receiver.attach(None, move |text: String| {
            let text_;
            if text != "Publish sent successfully\n" && text != "Connected successfully\n" {
                text_ = label.text().to_string() + "\n" + &text;
            } else {
                text_ = text;
            }
            label.set_text(text_.as_str());
            glib::Continue(true)
        });
    }

    pub fn update_subs_upon_publish(
        &self,
        glib_receiver: glib::Receiver<String>,
        label: gtk::Label,
    ) {
        glib_receiver.attach(None, move |text: String| {
            let topic_compare = "\n".to_string() + &text + "\n";
            if label.text().matches(&topic_compare).count() == 0 {
                let text = label.text().to_string() + &text + "\n";
                label.set_text(text.as_str());
            }
            glib::Continue(true)
        });
    }

    pub fn start(&mut self, glib_sender: glib::Sender<String>, receiver: Receiver<String>) {
        thread::spawn(move || loop {
            let text = receiver.recv();
            glib_sender
                .send(text.unwrap())
                .expect("Couldn't send data to channel");
        });
    }
}
