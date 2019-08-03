extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::datamodel;
use crate::component;

use datamodel::*;
use component::*;

use gio::prelude::*;
use gtk::{prelude::*, Widget, Button, Label, Container};
use std::env::args;
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

pub struct AppState {
    pub data: DataModel,
    pub async_request: Arc<Mutex<RequestStatus>>,
    ui_tree: Option<Component>,
    gui: GuiState
}

pub type AppPtr = Rc<RefCell<AppState>>;

struct GuiState {
    window: Rc<gtk::ApplicationWindow>,
    widgets: WidgetMap,
}

fn sign_in_page(_: AppPtr) -> Component {
    Component::new_leaf(Box::new(Factory::<Button>::new()))
        .with_attributes(map!("label" => "Sign in!"))
        .with_callback("clicked", sign_in_cb())
}

fn user_page(state: AppPtr) -> Component {
    let label = Component::new_leaf(Box::new(Factory::<Label>::new()))
        .with_attributes(map!("text" => "You are signed in!"));
    let button = Component::new_leaf(Box::new(Factory::<Button>::new()))
        .with_attributes(map!("label" => "Get transactions"))
        .with_callback("clicked", get_transactions_cb());
    let v = vec![label, button];
    Component::new_node(v, state, None)
}

fn main_app(state: AppPtr) -> Component {
    let mut v = Vec::new();
    if state.borrow().data.signed_in {
        v.push(sign_in_page as ComponentFn);
    }
    else {
        v.push(user_page as ComponentFn);
    }
    Component::new_node(v, state, None)
}

impl GuiState {
    fn new(app: &gtk::Application) -> GuiState {
        let window = gtk::ApplicationWindow::new(app);
        window.set_title("First GTK+ Program");
        window.set_border_width(10);
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(350, 70);
        GuiState {
            window: Rc::new(window),
            widgets: HashMap::new()
        }
    }
}

pub fn build_ui(state: AppPtr) {
    let app_tree = main_app(Rc::clone(&state));
    app_tree.render_diff(
        state.borrow().ui_tree.as_ref(), 
        state.borrow().gui.window.upcast_ref::<Container>(),
        &mut state.borrow_mut().gui.widgets,
        &state);
    state.borrow_mut().ui_tree = Some(app_tree);
}

pub fn run_app() {
    let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");
    application.connect_activate(move |app| {
        let app_state = AppState {
            data: DataModel::new(),
            async_request: Arc::new(Mutex::new(RequestStatus::NoReq)),
            ui_tree: None,
            gui: GuiState::new(app)
        };
        let app_ptr = Rc::new(RefCell::new(app_state));
        build_ui(app_ptr);
    });

    application.run(&args().collect::<Vec<_>>());
}