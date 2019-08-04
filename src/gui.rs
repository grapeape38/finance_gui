extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::datamodel;
use crate::component;

use datamodel::*;
use component::*;

use gio::prelude::*;
use gtk::{prelude::*, Widget, Window, Button, Label, Container};
use std::env::args;
use component::EWidget::*;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::collections::{HashMap};

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

macro_rules! c_map(
    { $($key:expr => $value:ty),+ } => {
        {
            let mut m = HashMap::new();
            $(
                m.insert($key, MyWidgetInfo::new(Box::new(Factory::<$value>::new())));
            )+
            m
        }
     };
);

pub struct AppState {
    pub data: RefCell<DataModel>,
    pub async_request: Arc<Mutex<RequestStatus>>,
    ui_tree: RefCell<Option<Component>>,
    pub widgets: RefCell<WidgetMap>
}

pub type AppPtr = Rc<AppState>;

impl AppState {
    fn new_ptr(app: &gtk::Application) -> AppPtr {
        let window = gtk::ApplicationWindow::new(app);
        window.set_title("First GTK+ Program");
        window.set_border_width(10);
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(350, 70);

        let mut widgets = create_widgets();
        widgets.get_mut(&MainWindow).unwrap().set(window.upcast::<Widget>());
        let app_state = AppState {
            data: RefCell::new(DataModel::new()),
            async_request: Arc::new(Mutex::new(RequestStatus::NoReq)),
            ui_tree: RefCell::new(None),
            widgets: RefCell::new(widgets)
        };
        Rc::new(app_state)
    }
}

fn create_widgets() -> HashMap<EWidget, MyWidgetInfo> {
    c_map!(
        SignInButton => Button,
        SignedInLabel => Label,
        GetTransButton => Button,
        MainWindow => Window
    )
}

fn sign_in_page(_: AppPtr) -> Component {
    Component::new_leaf(SignInButton)
        .with_attributes(map!("label" => "Sign in!"))
        .with_callback("clicked", sign_in_cb())
}

fn user_page(state: AppPtr) -> Component {
    let label = Component::new_leaf(SignedInLabel)
        .with_attributes(map!("text" => "You are signed in!"));
    let button = Component::new_leaf(GetTransButton)
        .with_attributes(map!("label" => "Get transactions"))
        .with_callback("clicked", get_transactions_cb());
    let v = vec![label, button];
    Component::new_node(v, state, None, "user_page")
}

fn main_app(state: AppPtr) -> Component {
    let mut v = Vec::new();
    if !state.data.borrow().signed_in {
        v.push(sign_in_page as ComponentFn);
    }
    else {
        v.push(user_page as ComponentFn);
    }
    Component::new_node(v, state, None, "main_app")
}

pub fn build_ui(state: AppPtr) {
    let app_tree = main_app(Rc::clone(&state));
    app_tree.render_diff(
        state.ui_tree.borrow().as_ref(),
        &MainWindow,
        &mut state.widgets.borrow_mut(),
        &state);
    *state.ui_tree.borrow_mut() = Some(app_tree);
}

pub fn run_app() {
    let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");
    application.connect_activate(move |app| {
        let app_state = AppState::new_ptr(app);
        build_ui(app_state);
    });

    application.run(&args().collect::<Vec<_>>());
}