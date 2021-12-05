extern crate glib;
extern crate gtk;
use self::gtk::atk::glib::clone;
use crate::packet::_send_disconnect_packet;
use crate::publish_interface::build_publish_ui;
use crate::publish_interface::ReceiverWindow;
use crate::subscription_interface::build_subscription_ui;
use crate::{client_run, FlagsConexion, UserInformation};
use gtk::prelude::*;
use std::net::TcpStream;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

pub fn run_connection_window() {
    let application = gtk::Application::new(Some("taller.Fra2ManEze"), Default::default());

    application.connect_activate(|app| {
        let _window = gtk::ApplicationWindow::new(app);
        build_connection_ui(app);
    });
    application.run();
}

fn build_connection_ui(app: &gtk::Application) {
    let connect_glade_src = include_str!("interface.glade");
    let connect_builder = gtk::Builder::from_string(connect_glade_src);

    let connect_window: gtk::Window = connect_builder.object("app_window").unwrap();
    connect_window.set_application(Some(app));
    let ip_entry: gtk::Entry = connect_builder.object("ip_entry").unwrap();
    let port_entry: gtk::Entry = connect_builder.object("port_entry").unwrap();
    let client_id_entry: gtk::Entry = connect_builder.object("client_id_entry").unwrap();
    let clean_session_check: gtk::ToggleButton =
        connect_builder.object("clean_session_check").unwrap();
    let username_entry: gtk::Entry = connect_builder.object("username_entry").unwrap();
    let password_entry: gtk::Entry = connect_builder.object("password_entry").unwrap();
    let will_message_entry: gtk::Entry = connect_builder.object("will_message_entry").unwrap();
    let will_topic_entry: gtk::Entry = connect_builder.object("will_topic_entry").unwrap();
    let connect_button: gtk::Button = connect_builder.object("connect_button").unwrap();
    let client_id_label: gtk::Label = connect_builder.object("client_id_label").unwrap();
    let user_pass_label: gtk::Label = connect_builder.object("user_pass_label").unwrap();
    let connection_label: gtk::Label = connect_builder.object("connection_label").unwrap();
    let ip_label: gtk::Label = connect_builder.object("ip_label").unwrap();
    let port_label: gtk::Label = connect_builder.object("port_label").unwrap();
    let disconnect_button: gtk::Button = connect_builder.object("disconnect_button").unwrap();
    let qos_will_switch: gtk::Switch = connect_builder.object("QOS_will_switch").unwrap();
    disconnect_button.hide();
    let app_destroy_clone = app.clone();
    connect_window.connect_destroy(move |_w| {
        app_destroy_clone.quit();
    });
    let app_clone = app.clone();
    connect_button.connect_clicked(clone!(@weak qos_will_switch, @weak disconnect_button, @weak connect_button, @weak ip_entry, @weak port_entry, @weak client_id_entry, @weak username_entry, @weak password_entry, @weak will_message_entry, @weak will_topic_entry, @weak client_id_label, @weak user_pass_label, @weak connection_label, @weak ip_label, @weak port_label, @weak clean_session_check => move |_|{
        let ip = ip_entry.text();
        let port = port_entry.text();
        let client_id = client_id_entry.text();
        let clean_session = clean_session_check.is_active();
        let username = username_entry.text();
        let password = password_entry.text();
        let will_message = will_message_entry.text();
        let will_topic = will_topic_entry.text();
        let mut will_qos:u8 = 0;
        if qos_will_switch.is_active(){
            will_qos = 1;
        }

        if client_id.len() != 0 && user_and_password_correct(username.as_str(), password.as_str()) && ip.len() != 0 && port.len() != 0{
            client_id_label.set_text("");
            user_pass_label.set_text("");
            ip_label.set_text("");
            port_label.set_text("");
            let address = ip.to_string() + ":" + &port;
            let user = UserInformation {
                id_length: client_id.len() as u16,
                id: client_id.to_string(),
                username_length: username.len() as u16,
                username: Some(username.to_string()),
                password_length: password.len() as u16,
                password: Some(password.to_string()),
                will_topic_length: will_topic.len() as u16,
                will_topic: Some(will_topic.to_string()),
                will_message_length: will_message.len() as u16,
                will_message: Some(will_message.to_string()),
                will_qos,
                keep_alive: 0,
            };
            let flags = FlagsConexion {
                username: username.len() > 0 ,
                password: password.len() > 0,
                will_retain: false,
                will_flag: will_topic.len() > 0,
                clean_session,
            };
            match TcpStream::connect(&address){
                Ok(tcpstream) => {
                    connection_label.set_text("");
                    println!("Connecting to... {:?}", &address);
                    let mut stream = tcpstream;
                    let stream_clone = stream.try_clone().unwrap();
                    let (puback_sender, puback_receiver): (Sender<String>, Receiver<String>) =
                    mpsc::channel();
                    let (message_sender, message_receiver): (Sender<String>, Receiver<String>) =
                    mpsc::channel();
                    let (connack_sender, connack_receiver): (Sender<String>, Receiver<String>) =
                    mpsc::channel();
                    let mut connack_window = ReceiverWindow::new().unwrap();
                    let (con_sender, con_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
                    connack_window.build(&connect_builder, con_receiver, "connection_succ_label");
                    connack_window.start(con_sender, connack_receiver);
                    let app_clone_2 = app_clone.clone();
                    let  stream_clone_2 = stream_clone.try_clone().expect("Cannot clone stream");
                    disconnect_button.connect_clicked(move |_| {
                        let mut stream_clone3 = stream_clone_2.try_clone().expect("Cannot clone stream");
                        _send_disconnect_packet(&mut stream_clone3);
                        app_clone_2.quit();
                    });
                    thread::spawn(move| |{
                        client_run(stream_clone, user, flags, connack_sender, puback_sender, message_sender).unwrap();
                    });

                    build_publish_ui(&mut stream, client_id.to_string(), puback_receiver, &connect_builder);
                    build_subscription_ui(&mut stream, message_receiver, &connect_builder);
                    connect_button.hide();
                    disconnect_button.show();
                    ip_entry.set_properties(&[("can-focus", &false)]).unwrap();
                    port_entry.set_properties(&[("can-focus", &false)]).unwrap();
                    client_id_entry.set_properties(&[("can-focus", &false)]).unwrap();
                    username_entry.set_properties(&[("can-focus", &false)]).unwrap();
                    password_entry.set_properties(&[("can-focus", &false)]).unwrap();
                    will_message_entry.set_properties(&[("can-focus", &false)]).unwrap();
                    will_topic_entry.set_properties(&[("can-focus", &false)]).unwrap();
                }
                Err(_) => {connection_label.set_text("Connection failed");}
            }
        }
        else{
            check_if_not_empty(client_id.len() == 0, client_id_label, "Client ID is required");
            check_if_not_empty(!user_and_password_correct(username.as_str(), password.as_str()), user_pass_label, "Invalid username or password");
            check_if_not_empty(ip.len() == 0, ip_label, "IP is required");
            check_if_not_empty(port.len() == 0, port_label, "Port is required");
        }
    }));

    connect_window.show();
}

fn check_if_not_empty(condition: bool, label: gtk::Label, error: &str) {
    if condition {
        label.set_text(error);
    } else {
        label.set_text("");
    }
}

fn user_and_password_correct(user: &str, password: &str) -> bool {
    let file: String = match std::fs::read_to_string("../server/src/users.txt") {
        Ok(file) => file,
        Err(_) => return false,
    };
    let lines = file.lines();

    for line in lines {
        let name_and_pass: Vec<&str> = line.split('=').collect();
        let username: String = name_and_pass[0].to_string();
        let pass: String = name_and_pass[1].to_string();
        if username == user {
            if pass == password {
                return true;
            }
            return false;
        }
    }
    false
}
