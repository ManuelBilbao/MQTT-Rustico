extern crate glib;
extern crate gtk;
use self::gtk::atk::glib::clone;
use crate::packet::_send_disconnect_packet;
use crate::publish_interface::build_publish_ui;
use crate::publish_interface::ReceiverObject;
use crate::subscription_interface::build_subscription_ui;
use crate::{client_run, FlagsConnection, UserInformation};
use gtk::prelude::*;
use rand::Rng;
use std::net::TcpStream;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
/// Runs the interface where the first window is the connection window
pub fn run_connection_window() {
    let mut rng = rand::thread_rng();
    let name_id: u16 = rng.gen();
    let name = "taller.Fra2ManEze".to_string() + &name_id.to_string();
    let application = gtk::Application::new(Some(&name), Default::default());

    application.connect_activate(|app| {
        build_connection_ui(app);
    });
    application.run();
}

fn build_connection_ui(app: &gtk::Application) {
    let connect_glade_src = include_str!("interface.glade");
    let connect_builder = gtk::Builder::from_string(connect_glade_src);

    let connect_window: gtk::Window = connect_builder
        .object("app_window")
        .expect("Error when getting window from glade");
    connect_window.set_application(Some(app));

    let app_destroy_clone = app.clone();
    connect_window.connect_destroy(move |_w| {
        app_destroy_clone.quit();
    });
    run_connection(app, connect_builder);
    connect_window.show();
}
/// The disconnect button will send a disconnect packet to the server when clicked.
fn disconnect_on_click(
    app: &gtk::Application,
    stream: &mut TcpStream,
    disconnect_button: &gtk::Button,
) {
    let app_clone = app.clone();
    let stream_clone = stream.try_clone().expect("Cannot clone stream");
    disconnect_button.connect_clicked(move |_| {
        let mut stream_clone_2 = stream_clone.try_clone().expect("Cannot clone stream");
        _send_disconnect_packet(&mut stream_clone_2);
        app_clone.quit();
    });
}
/// Creates mpsc channels to communicate with the client (the client keeps the channel sender), spawns a thread to run the client and builds the publish window and subscription window.
fn run_client_and_build_windows(
    mut stream: TcpStream,
    app: &gtk::Application,
    disconnect_button: &gtk::Button,
    connect_builder: &gtk::Builder,
    user: UserInformation,
    flags: FlagsConnection,
) {
    let stream_clone = stream.try_clone().expect("Error when cloning stream");
    let (puback_sender, puback_receiver): (Sender<String>, Receiver<String>) = mpsc::channel();
    let (message_sender, message_receiver): (Sender<String>, Receiver<String>) = mpsc::channel();
    let (connack_sender, connack_receiver): (Sender<String>, Receiver<String>) = mpsc::channel();
    let (topic_update_sender, topic_update_receiver): (Sender<String>, Receiver<String>) =
        mpsc::channel();
    let mut connack_obj = ReceiverObject::new().expect("Error when creating ReceiverObject");
    let (con_sender, con_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    connack_obj.build(connect_builder, con_receiver, "connection_succ_label");
    connack_obj.start(con_sender, connack_receiver);
    disconnect_on_click(app, &mut stream, disconnect_button);
    thread::spawn(move || {
        client_run(
            stream_clone,
            user,
            flags,
            connack_sender,
            puback_sender,
            message_sender,
            topic_update_sender,
        )
        .expect("Error when running client");
    });
    build_publish_ui(&mut stream, puback_receiver, connect_builder);
    build_subscription_ui(
        &mut stream,
        message_receiver,
        topic_update_receiver,
        connect_builder,
    );
}

fn connect_and_run_client(
    address: &str,
    app: &gtk::Application,
    disconnect_button: gtk::Button,
    connect_button: gtk::Button,
    user: UserInformation,
    flags: FlagsConnection,
    connect_builder: &gtk::Builder,
) -> bool {
    let connection_label: gtk::Label = connect_builder
        .object("connection_label")
        .expect("Error when getting object from glade");
    match TcpStream::connect(&address) {
        Ok(tcpstream) => {
            connection_label.set_text("");
            println!("Connecting to... {:?}", &address);
            run_client_and_build_windows(
                tcpstream,
                app,
                &disconnect_button,
                connect_builder,
                user,
                flags,
            );

            connect_button.hide();
            disconnect_button.show();
            true
        }
        Err(_) => {
            connection_label.set_text("Connection failed");
            false
        }
    }
}
fn check_if_not_empty(condition: bool, label: gtk::Label, error: &str) {
    if condition {
        label.set_text(error);
    } else {
        label.set_text("");
    }
}

fn initialize_userinformation(
    client_id_entry: &gtk::Entry,
    username_entry: &gtk::Entry,
    password_entry: &gtk::Entry,
    will_topic_entry: &gtk::Entry,
    will_message_entry: &gtk::Entry,
    qos_will_switch: &gtk::Switch,
) -> UserInformation {
    UserInformation {
        id_length: client_id_entry.text().len() as u16,
        id: client_id_entry.text().to_string(),
        username_length: username_entry.text().len() as u16,
        username: Some(username_entry.text().to_string()),
        password_length: password_entry.text().len() as u16,
        password: Some(password_entry.text().to_string()),
        will_topic_length: will_topic_entry.text().len() as u16,
        will_topic: Some(will_topic_entry.text().to_string()),
        will_message_length: will_message_entry.text().len() as u16,
        will_message: Some(will_message_entry.text().to_string()),
        will_qos: qos_will_switch.is_active() as u8,
        keep_alive: 0,
    }
}

fn initialize_flags(
    username_entry: &gtk::Entry,
    password_entry: &gtk::Entry,
    will_retain_check: &gtk::ToggleButton,
    will_topic_entry: &gtk::Entry,
    clean_session_check: &gtk::ToggleButton,
) -> FlagsConnection {
    FlagsConnection {
        username: username_entry.text().len() > 0,
        password: password_entry.text().len() > 0,
        will_retain: will_retain_check.is_active(),
        will_flag: will_topic_entry.text().len() > 0,
        clean_session: clean_session_check.is_active(),
    }
}

fn set_properties(
    ip_entry: &gtk::Entry,
    port_entry: &gtk::Entry,
    client_id_entry: &gtk::Entry,
    username_entry: &gtk::Entry,
    password_entry: &gtk::Entry,
    will_message_entry: &gtk::Entry,
    will_topic_entry: &gtk::Entry,
) {
    ip_entry
        .set_properties(&[("can-focus", &false)])
        .expect("Error when setting properties");
    port_entry
        .set_properties(&[("can-focus", &false)])
        .expect("Error when setting properties");
    client_id_entry
        .set_properties(&[("can-focus", &false)])
        .expect("Error when setting properties");
    username_entry
        .set_properties(&[("can-focus", &false)])
        .expect("Error when setting properties");
    password_entry
        .set_properties(&[("can-focus", &false)])
        .expect("Error when setting properties");
    will_message_entry
        .set_properties(&[("can-focus", &false)])
        .expect("Error when setting properties");
    will_topic_entry
        .set_properties(&[("can-focus", &false)])
        .expect("Error when setting properties");
}
fn get_info_objects(
    connect_builder: &gtk::Builder,
) -> (
    gtk::Entry,
    gtk::Entry,
    gtk::Entry,
    gtk::Entry,
    gtk::Switch,
    gtk::ToggleButton,
    gtk::ToggleButton,
) {
    let username_entry: gtk::Entry = connect_builder
        .object("username_entry")
        .expect("Error when getting object from glade");
    let password_entry: gtk::Entry = connect_builder
        .object("password_entry")
        .expect("Error when getting object from glade");
    let will_message_entry: gtk::Entry = connect_builder
        .object("will_message_entry")
        .expect("Error when getting object from glade");
    let will_topic_entry: gtk::Entry = connect_builder
        .object("will_topic_entry")
        .expect("Error when getting object from glade");
    let qos_will_switch: gtk::Switch = connect_builder
        .object("QOS_will_switch")
        .expect("Error when getting object from glade");
    let will_retained_check: gtk::ToggleButton = connect_builder
        .object("will_retained_check")
        .expect("Error when getting object from glade");
    let clean_session_check: gtk::ToggleButton = connect_builder
        .object("clean_session_check")
        .expect("Error when getting object from glade");

    (
        username_entry,
        password_entry,
        will_message_entry,
        will_topic_entry,
        qos_will_switch,
        will_retained_check,
        clean_session_check,
    )
}

fn get_info_and_flags(
    entries: &(gtk::Entry, gtk::Entry, gtk::Entry),
    connect_builder: &gtk::Builder,
) -> (UserInformation, FlagsConnection) {
    let info_objects = get_info_objects(connect_builder);
    let user = initialize_userinformation(
        &entries.2,
        &info_objects.0,
        &info_objects.1,
        &info_objects.3,
        &info_objects.2,
        &info_objects.4,
    );
    let flags = initialize_flags(
        &info_objects.0,
        &info_objects.1,
        &info_objects.5,
        &info_objects.3,
        &info_objects.6,
    );
    (user, flags)
}

fn try_connection(
    address: String,
    app: &gtk::Application,
    disconnect_button: gtk::Button,
    connect_button: gtk::Button,
    connect_builder: &gtk::Builder,
    entries: (gtk::Entry, gtk::Entry, gtk::Entry),
) {
    let info_flags = get_info_and_flags(&entries, connect_builder);
    if connect_and_run_client(
        &address,
        app,
        disconnect_button,
        connect_button,
        info_flags.0,
        info_flags.1,
        connect_builder,
    ) {
        let info_objects = get_info_objects(connect_builder);
        set_properties(
            &entries.0,
            &entries.1,
            &entries.2,
            &info_objects.0,
            &info_objects.1,
            &info_objects.2,
            &info_objects.3,
        );
    }
}

fn initialize_client_and_connect(
    connect_builder: &gtk::Builder,
    app: &gtk::Application,
    disconnect_button: gtk::Button,
    connect_button: gtk::Button,
    ip_entry: gtk::Entry,
    port_entry: gtk::Entry,
    client_id_entry: gtk::Entry,
) {
    let ip_label: gtk::Label = connect_builder
        .object("ip_label")
        .expect("Error when getting object from glade");
    let port_label: gtk::Label = connect_builder
        .object("port_label")
        .expect("Error when getting object from glade");
    let client_id_label: gtk::Label = connect_builder
        .object("client_id_label")
        .expect("Error when getting object from glade");
    if client_id_entry.text().len() != 0
        && ip_entry.text().len() != 0
        && port_entry.text().len() != 0
    {
        client_id_label.set_text("");
        ip_label.set_text("");
        port_label.set_text("");
        let address = ip_entry.text().to_string() + ":" + &port_entry.text();
        try_connection(
            address,
            app,
            disconnect_button,
            connect_button,
            connect_builder,
            (ip_entry, port_entry, client_id_entry),
        );
    } else {
        check_if_not_empty(
            client_id_entry.text().len() == 0,
            client_id_label,
            "Client ID is required",
        );
        check_if_not_empty(ip_entry.text().len() == 0, ip_label, "IP is required");
        check_if_not_empty(port_entry.text().len() == 0, port_label, "Port is required");
    }
}

fn get_objects_and_connect(
    connect_builder: &gtk::Builder,
    app: &gtk::Application,
    disconnect_button: gtk::Button,
    connect_button: gtk::Button,
) {
    let ip_entry: gtk::Entry = connect_builder
        .object("ip_entry")
        .expect("Error when getting object from glade");
    let port_entry: gtk::Entry = connect_builder
        .object("port_entry")
        .expect("Error when getting object from glade");
    let client_id_entry: gtk::Entry = connect_builder
        .object("client_id_entry")
        .expect("Error when getting object from glade");

    initialize_client_and_connect(
        connect_builder,
        app,
        disconnect_button,
        connect_button,
        ip_entry,
        port_entry,
        client_id_entry,
    );
}
/// When the connect button is clicked, it connects and runs the client and then builds the publish window and subscription window.
fn run_connection(app: &gtk::Application, connect_builder: gtk::Builder) {
    let disconnect_button: gtk::Button = connect_builder
        .object("disconnect_button")
        .expect("Error when getting object from glade");
    let connect_button: gtk::Button = connect_builder
        .object("connect_button")
        .expect("Error when getting object from glade");
    disconnect_button.hide();

    let app_clone = app.clone();
    connect_button.connect_clicked(clone!(@weak disconnect_button, @weak connect_button => move |_|{
        get_objects_and_connect(&connect_builder, &app_clone, disconnect_button, connect_button);
    }));
}
