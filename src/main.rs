extern crate gio;
extern crate gtk;
extern crate hyper;
use hyper::rt::{self, Future, Stream};

use gio::prelude::*;
use gtk::prelude::*;

use std::env::args;
use std::error::Error;
use std::sync::{Arc, Mutex};

mod plaid;
use plaid::{ClientHandle, get_access_token, API_VERSION};

use tokio_timer::{sleep};
use hyper::header::{HeaderValue};

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title("First GTK+ Program");
    window.set_border_width(10);
    window.set_position(gtk::WindowPosition::Center);
    window.set_default_size(600, 480);


    let button = gtk::Button::new_with_label("Sign in");
    button.connect_clicked(|_| {
        println!("Clicked!");
    });

    window.add(&button);

    window.show_all();
}

fn run() -> Result<(), Box<Error>> {
    rt::run(rt::lazy(|| {
        let ch = ClientHandle::new().unwrap();
        //let ch_mut = Arc::new(Mutex::new(ch));
        get_access_token(ch).and_then(|mut ch| {
            let access_token = ch.auth_params.access_token.clone().unwrap(); 
            let item_id = ch.auth_params.item_id.clone().unwrap(); 
            println!("Access Token: {}, Item Id: {}", access_token, item_id);
            ch.headers.insert("Plaid-Version", HeaderValue::from_static(API_VERSION));
            ch.headers.remove("Plaid-Link-Version");
            sleep(std::time::Duration::from_millis(1000))
                .map_err(|e| panic!("{}", e)).and_then(|_| ch.get_transactions())
        }).and_then(|ch| {
            println!("{}", ch.result);
            Ok(())
        }).map_err(|e| {
            panic!("Error: {}", e)
        })
    }));
    Ok(())
}

fn main() {
    /*let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());*/
    match run() {
        Ok(()) => {}
        Err(e) => println!("Error: {}", e)
    };
}