extern crate gio;
extern crate gtk;

use gio::prelude::*;
use gtk::prelude::*;

use std::env::args;

mod plaid;
use plaid::{run_plaid};

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title("First GTK+ Program");
    window.set_border_width(10);
    window.set_position(gtk::WindowPosition::Center);
    window.set_default_size(350, 70);


    let button = gtk::Button::new_with_label("Click me!");
    button.connect_clicked(|_| {
        println!("Clicked!");
    });

    window.add(&button);

    window.show_all();
}

fn main() {
    match run_plaid() {
        Ok(()) => {}
        Err(e) => println!("Error: {}", e)
    };
    /*
    let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());*/
}