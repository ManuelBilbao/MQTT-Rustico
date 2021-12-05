extern crate glib;
extern crate gtk;
use self::gtk::atk::glib::clone;
use crate::packet::{_send_subscribe_packet, _send_unsubscribe_packet};
use crate::publish_interface::ReceiverWindow;
use gtk::prelude::*;
use std::net::TcpStream;
use std::sync::mpsc::Receiver;

pub fn build_subscription_ui(
    stream: &mut TcpStream,
    message_receiver: Receiver<String>,
    connect_builder: &gtk::Builder,
) {
    let mut sub_window_2 = ReceiverWindow::new().unwrap();
    let (sub_sender_2, sub_receiver_2) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let subscribe_entry: gtk::Entry = connect_builder.object("subscribe_entry").unwrap();
    let unsubscribe_entry: gtk::Entry = connect_builder.object("unsubscribe_entry").unwrap();
    let subscribe_button: gtk::Button = connect_builder.object("subscribe_button").unwrap();
    let unsubscribe_button: gtk::Button = connect_builder.object("unsubscribe_button").unwrap();

    let stream_clone = stream.try_clone().expect("Cannot clone stream");
    subscribe_button.connect_clicked(clone!(@weak subscribe_entry => move |_|{
        let topic = subscribe_entry.text();
        let topic_vec = vec!(topic.to_string());

        let mut stream_clone3 = stream_clone.try_clone().expect("Cannot clone stream");
        _send_subscribe_packet(&mut stream_clone3, topic_vec);
        subscribe_entry.set_properties(&[("text", &"".to_owned())]).unwrap();
    }));
    let stream_clone2 = stream.try_clone().expect("Cannot clone stream");
    unsubscribe_button.connect_clicked(clone!(@weak unsubscribe_entry => move |_|{
        let topic = unsubscribe_entry.text();
        let topic_vec = vec!(topic.to_string());

        let mut stream_clone3 = stream_clone2.try_clone().expect("Cannot clone stream");
        _send_unsubscribe_packet(&mut stream_clone3, topic_vec);
        unsubscribe_entry.set_properties(&[("text", &"".to_owned())]).unwrap();
    }));
    sub_window_2.build(connect_builder, sub_receiver_2, "message_label");
    sub_window_2.start(sub_sender_2, message_receiver);
}
