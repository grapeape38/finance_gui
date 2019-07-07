extern crate gio;
extern crate gtk;
extern crate rand;

use rand::{Rng};

use gio::prelude::*;
use gtk::prelude::*;

use std::env::args;

mod plaid;
use plaid::makeRequest;

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

fn getRandomID() -> String {
    let mut rng = rand::thread_rng();
    let rand_ints = (0..16).map(|_| rng.gen_range(0,256));
    let hex_bytes : Vec<String> = (256..512).map(|x| format!("{:x}", x)).collect();
    let m : Vec<&str> = rand_ints.map(|x| &hex_bytes[x][1..]).collect();
    return format!("{}{}{}{}-{}{}-{}{}-{}{}-{}{}{}{}{}{}", m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10], m[11], m[12],m[13],m[14],m[15]); 
}

fn main() {
    makeRequest();
    /*
    let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());*/
}