extern crate glib;
extern crate gtk;
use self::gtk::atk::glib::clone;
use crate::packet::{send_subscribe_packet, send_unsubscribe_packet};
use crate::publish_interface::ReceiverObject;
use gtk::prelude::*;
use std::net::TcpStream;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

pub fn build_subscription_ui(
    stream: &mut TcpStream,
    message_receiver: Receiver<String>,
    topic_update_receiver: Receiver<String>,
    connect_builder: &gtk::Builder,
) {
    let mut sub_obj = ReceiverObject::new().expect("Error when creating ReceiverObject");
    let mut topics_updater = ReceiverObject::new().expect("Error when creating ReceiverObject");
    let (sub_sender_2, sub_receiver_2) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    let (sub_topic_sender, sub_topic_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let subscribe_entry: gtk::Entry = connect_builder
        .object("subscribe_entry")
        .expect("Error when getting object from glade");
    let unsubscribe_entry: gtk::Entry = connect_builder
        .object("unsubscribe_entry")
        .expect("Error when getting object from glade");
    let subscribe_button: gtk::Button = connect_builder
        .object("subscribe_button")
        .expect("Error when getting object from glade");
    let qos_subscribe_switch: gtk::Switch = connect_builder
        .object("QOS_subscribe_switch")
        .expect("Error when getting object from glade");
    let unsubscribe_button: gtk::Button = connect_builder
        .object("unsubscribe_button")
        .expect("Error when getting object from glade");
    let current_subscriptions_label: gtk::Label = connect_builder
        .object("current_subscriptions_label")
        .expect("Error when getting object from glade");
    let subs_lock = Arc::new(Mutex::new(current_subscriptions_label));
    let subs_lock_2 = subs_lock.clone();
    let subs_lock_3 = subs_lock.clone();

    let stream_clone = stream.try_clone().expect("Cannot clone stream");
    subscribe_button_on_click(
        subscribe_button,
        stream_clone,
        subscribe_entry,
        subs_lock,
        qos_subscribe_switch,
    );
    let stream_clone_2 = stream.try_clone().expect("Cannot clone stream");
    unsubscribe_button_on_click(
        unsubscribe_button,
        stream_clone_2,
        unsubscribe_entry,
        subs_lock_2,
    );

    sub_obj.build(connect_builder, sub_receiver_2, "message_label");
    sub_obj.start(sub_sender_2, message_receiver);
    topics_updater.update_subs_upon_publish(sub_topic_receiver, subs_lock_3);
    topics_updater.start(sub_topic_sender, topic_update_receiver);
}
/// Sends subscribe packet and updates label when subscribe button is clicked
fn subscribe_button_on_click(
    subscribe_button: gtk::Button,
    stream: TcpStream,
    subscribe_entry: gtk::Entry,
    current_subscriptions_lock: Arc<Mutex<gtk::Label>>,
    qos_subscribe_switch: gtk::Switch,
) {
    subscribe_button.connect_clicked(clone!(@weak subscribe_entry, @weak qos_subscribe_switch => move |_|{
        let topic = subscribe_entry.text();
        let topic_vec = vec!(topic.to_string());
        let topic_compare = "\n".to_string() + &topic + "\n";
        match current_subscriptions_lock.lock() {
                Ok(current_subscriptions_label) => {
                    if current_subscriptions_label.text().matches(&topic_compare).count() == 0{
                        let text = current_subscriptions_label.text().to_string() + &topic + "\n";
                        current_subscriptions_label.set_text(text.as_str());
                    }
                    let mut stream_clone3 = stream.try_clone().expect("Cannot clone stream");
                    send_subscribe_packet(&mut stream_clone3, topic_vec, qos_subscribe_switch.is_active());
                    subscribe_entry.set_properties(&[("text", &"".to_owned())]).expect("Error when setting properties");
                }
                Err(_) => {
                    println!("Error when adding to current subscriptions");
                }
        }
    }));
}
///Sends unsubscribe packet and updates label when unsubscribe button is clicked
fn unsubscribe_button_on_click(
    unsubscribe_button: gtk::Button,
    stream: TcpStream,
    unsubscribe_entry: gtk::Entry,
    current_subscriptions_lock: Arc<Mutex<gtk::Label>>,
) {
    unsubscribe_button.connect_clicked(clone!(@weak unsubscribe_entry => move |_|{
        let topic = unsubscribe_entry.text();
        let topic_vec = vec!(topic.to_string());
        let topic_compare = "\n".to_string() + &topic + "\n";
        match current_subscriptions_lock.lock() {
                Ok(current_subscriptions_label) => {
                    if current_subscriptions_label.text().matches(&topic_compare).count() > 0{
                        let text = str::replace(&current_subscriptions_label.text(), &topic_compare, "\n");
                        current_subscriptions_label.set_text(text.as_str());
                        let mut stream_clone = stream.try_clone().expect("Cannot clone stream");
                        send_unsubscribe_packet(&mut stream_clone, topic_vec);
                        unsubscribe_entry.set_properties(&[("text", &"".to_owned())]).expect("Error when setting properties");
                    }
                }
                Err(_) => {
                    println!("Error when deleting from current subscriptions");
                }
        }
    }));
}
