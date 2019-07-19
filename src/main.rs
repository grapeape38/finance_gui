extern crate gio;
extern crate gtk;
extern crate hyper;
use hyper::rt::{self, Future, Stream};

use gio::prelude::*;
use gtk::prelude::*;

use std::env::args;
use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};
use std::collections::HashMap;

mod plaid;
use plaid::{ClientHandle, get_access_token, AuthParams, API_VERSION};

use tokio_timer::{sleep};
use hyper::header::{HeaderValue};

/*struct AppState<'a> {
    application: &'a gtk::Application,
    window: Arc<Mutex<gtk::ApplicationWindow>>,
    buttons: HashMap<String, gtk::Button>
    auth_params: Arc<Mutex<gtk::AuthParams>>
};*/

fn build_ui(application: &gtk::Application) /*-> AppState*/ {
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

fn run() -> Result<AuthParams, Box<Error>> {
    let auth_mut = Arc::new(Mutex::new(AuthParams::new()?));
    let auth_mut2 = Arc::clone(&auth_mut);
    rt::run(rt::lazy(move || {
        /*let application =
            gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
                .expect("Initialization failed...");

        application.connect_activate(|app| {
            build_ui(app);
        });

        application.connect_shutdown(|_| {
            println!("I am shutting down!");
        });

        application.run(&args().collect::<Vec<_>>());*/

        let ch = ClientHandle::new().unwrap();

        get_access_token(ch).and_then(move |ch| {
            let access_token = ch.auth_params.access_token.clone().unwrap(); 
            let item_id = ch.auth_params.item_id.clone().unwrap(); 
            *auth_mut.lock().unwrap() = ch.auth_params;
            Ok(())
            /*
            println!("Access Token: {}, Item Id: {}", access_token, item_id);
            sleep(std::time::Duration::from_millis(1000))
                .map_err(|e| panic!("{}", e)).and_then(|_| ch.get_transactions())
            }).and_then(|ch| {
                println!("{}", ch.result);
                Ok(())*/
            }).map_err(|e| {
            panic!("Error: {}", e)
        })
    }));
    Ok(Arc::try_unwrap(auth_mut2).unwrap().into_inner().unwrap())
}

fn main() {
    match run() {
        Ok(auth_params) => { println!("Access token: {}", auth_params.access_token.unwrap()); }
        Err(e) => println!("Error: {}", e)
    };
}