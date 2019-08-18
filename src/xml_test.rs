extern crate gio;
extern crate gtk;
extern crate xml;

use gio::prelude::*;
use gtk::{prelude::*, Widget, Container, Application, ApplicationWindow};
use std::env::args;
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use crate::component2::*;

use std::rc::Rc;
use std::cell::RefCell;

macro_rules! map(
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);

pub struct AppState {
    pub widget_map: RefCell<WidgetMap>,
    pub ui_tree: RefCell<Component>,
    pub window: ApplicationWindow
}

impl AppState {
    fn new_ptr(app: &Application) -> AppPtr {
        let window = gtk::ApplicationWindow::new(app);
        Rc::new(AppState {
            widget_map: RefCell::new(HashMap::new()),
            ui_tree: RefCell::new(Component::empty()),
            window
        })
    }
}

pub type WidgetMap = HashMap<String, Widget>;

pub type AppPtr = Rc<AppState>;

fn build_ui(app: &AppPtr) {
    /*let xml_string = "
        <interface>
            <object class='GtkBox' id='main_box'>
                <property name='orientation'>GTK_ORIENTATION_HORIZONTAL</property>
                <property name='spacing'>4</property>
                <child>
                <object class='GtkButton' id='hello_button'>
                    <property name='label'>Hello World</property>
                    <signal name='clicked' handler='hello_button_clicked' object='FooWidget' swapped='yes'/>
                </object>
                </child>
                <child>
                <object class='GtkButton' id='goodbye_button'>
                    <property name='label'>Goodbye World</property>
                </object>
                </child>
            </object>
        </interface>";*/
    let window = &app.window;
    window.set_title("Finance Viewer App");
    window.set_border_width(10);
    window.set_position(gtk::WindowPosition::Center);
    window.set_default_size(350, 70);

    let frame = new_comp::<gtk::Frame>("my_frame").with_props(map!("label" => "Test label"));
    let button1 = new_comp::<gtk::Button>("hello_button").with_props(map!("label" => "Hello World"));
    let button2  = new_comp::<gtk::Button>("goodbye_button").with_props(map!("label" => "Goodbye World"));
    let root = new_comp::<gtk::Box>("main_box").with_props(map!("orientation" => "GTK_ORIENTATION_HORIZONTAL", "spacing" => "4"))
        .with_children(vec![frame, button1, button2]);

    println!("XML:\n{}", root.to_xml_string().expect("Error serializing to xml!"));

    let mainbox = root.build(&mut app.widget_map.borrow_mut());

    window.add(&mainbox);
    window.show_all();
}

fn rebuild(app: &AppPtr) {
    let ui2 = new_comp::<gtk::Button>("mybutton");
    if app.ui_tree.borrow().id != ui2.id {
        remove_widget_maybe(&ui2.id, app);
        let child = ui2.build(&mut app.widget_map.borrow_mut());
        add_child_maybe(&child, app.window.upcast_ref::<Container>());
        app.widget_map.borrow_mut().insert(ui2.id, child);
    }
    else {
        ui2.render_diff(&app.ui_tree.borrow(), app);
    }
}

fn test_main() {
    let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");
    application.connect_activate(move |app| {
        let ap = AppState::new_ptr(app);
        build_ui(&ap);
        /*sleep(Duration::from_millis(2000));
        rebuild(&ap);*/
    });
    application.run(&args().collect::<Vec<_>>());
}
