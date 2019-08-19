extern crate gio;
extern crate gtk;
extern crate xml;

use gio::prelude::*;
use gtk::{prelude::*, Widget, Container, Application, ApplicationWindow};
use std::env::args;
use std::collections::HashMap;
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
    let window = &app.window;
    window.set_title("Finance Viewer App");
    window.set_border_width(10);
    window.set_position(gtk::WindowPosition::Center);
    window.set_default_size(350, 70);

    let frame = Component::from_xml_string("
        <object class='GtkFrame' id='main_frame'>
            <child>
                <object class='GtkLabel' id='my_label'>
                    <property name='label'>What's up Text</property>
                </object>
            </child>
        </object>
    ").expect("Error parsing xml");

    let button1 = new_comp::<gtk::Button>("hello_button").with_props(map!("label" => "Hello World"));
    let button2  = new_comp::<gtk::Button>("goodbye_button").with_props(map!("label" => "Goodbye World"));
    let root = new_comp::<gtk::Box>("main_box").with_props(map!("orientation" => "GTK_ORIENTATION_HORIZONTAL", "spacing" => "4"))
        .with_children(vec![frame, button1, button2]);

    println!("XML:\n{}", root.to_xml_string().expect("Error serializing to xml!"));

    root.build(app);
    window.add(&app.widget_map.borrow()["main_box"]);
    *app.ui_tree.borrow_mut() = root;
    window.show_all();
}

fn testui1() -> Component {
    let button = new_comp::<gtk::Button>("mybutton").with_props(map!("label" => "First button haha!"));
    let mybox = new_comp::<gtk::Box>("mybox").with_children(vec![button]);
    mybox 
}

fn testui2() -> Component {
    let button = new_comp::<gtk::Button>("mybutton2").with_props(map!("label" => "Second button haha!"));
    let mybox = new_comp::<gtk::Box>("mybox").with_children(vec![button]);
    mybox 
}

fn rebuild(app: &AppPtr, new_ui: Component) {
    println!("New XML:\n{}", new_ui.to_xml_string().expect("Error serializing to xml!"));
    let old_id = app.ui_tree.borrow().id.clone();
    if old_id != new_ui.id {
        app.ui_tree.borrow().remove_self_widget(&mut app.widget_map.borrow_mut());
        new_ui.build(app);
        add_child_maybe(&app.widget_map.borrow()[&new_ui.id], app.window.upcast_ref::<Container>());
    }
    else {
        new_ui.render_diff(&app.ui_tree.borrow(), app);
    }
    *app.ui_tree.borrow_mut() = new_ui;
}

pub fn test_main() {
    let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");
    application.connect_activate(move |app| {
        let app_state = AppState::new_ptr(app);
        build_ui(&app_state);
        rebuild(&app_state, testui1());
        rebuild(&app_state, testui2());
    });
    application.run(&args().collect::<Vec<_>>());
}
