extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::datamodel;
use crate::component;
use crate::plaid;

use datamodel::*;
use component::*;
use plaid::Transaction;
use component::ComponentID::*;

use gio::prelude::*;
use gtk::{prelude::*, Widget, Window, Button, Label, Container};
use std::env::args;
use component::EWidget::*;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::collections::{HashMap};
use serde_json::Value;

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
    pub async_request: Arc<Mutex<ReqStatus<Value>>>,
    ui_tree: RefCell<Option<Component>>,
    pub widgets: WidgetMap
}

pub type AppPtr = Rc<AppState>;

impl AppState {
    fn new_ptr(app: &gtk::Application) -> AppPtr {
        let window = gtk::ApplicationWindow::new(app);
        window.set_title("Finance Viewer App");
        window.set_border_width(10);
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(350, 70);

        let mut widgets = create_widgets();
        widgets.get_mut(&MainWindow).unwrap().set(window.upcast::<Widget>(), 0);
        let app_state = AppState {
            data: RefCell::new(DataModel::new()),
            async_request: Arc::new(Mutex::new(Ok(RespType::None))),
            ui_tree: RefCell::new(None),
            widgets
        };
        Rc::new(app_state)
    }
}

fn create_widgets() -> WidgetMap {
    c_map!(
        SignInButton => Button,
        LoadingFrame => gtk::Frame,
        SignedInFrame => gtk::Frame,
        ErrorPage => gtk::Frame,
        GetTransButton => Button,
        MainWindow => Window,
        MainBox => gtk::Box,
        TransColLabel => gtk::Label,
        TransColBin => gtk::Frame,
        TransRow => gtk::Box,
        TransBox => gtk::Box
    )
}

fn trans_row(trans: &Transaction) -> Component {
    let amt = format!("{}", trans.amount);
    let entries = vec![
        &trans.account_id,
        &amt,
        &trans.date,
        &trans.name,
        &trans.transaction_id,
        &trans.transaction_type
    ];
    let mut i = 0;
    let rowvec = entries.into_iter().map(|entry| {
        let mut key = to_key(TransColLabel, format!("{}-{}", trans.transaction_id, i));
        let label = new_leaf(key).with_attributes(map!("text" => entry.clone()));
        i += 1;
        key = to_key(TransColBin, format!("{}-{}-bin", trans.transaction_id, i));
        new_node(vec![label], key)
    }).collect();
    new_node(rowvec, to_key(TransRow, &trans.transaction_id)).with_attributes(map!("orientation" => "horizontal".to_string()))
}

fn trans_box(transactions: &Vec<Transaction>) -> Component {
    let rows = transactions.iter().map(|t| {
        trans_row(t)
    }).collect();
    new_node(rows, TransBox)
}

fn sign_in_page(_: &AppPtr) -> Component {
    new_leaf(SignInButton)
        .with_attributes(map!("label" => "Sign in!".to_string()))
        .with_callback("clicked", sign_in_cb())
}

fn user_page(state: &AppPtr) -> Component {
    let mut v = Vec::new();
    let label_text = format!("You are signed in! Your access token is: {}", state.data.borrow().auth_params.access_token.as_ref().unwrap());
    let label = new_leaf(SignedInFrame)
        .with_attributes(map!("label" => label_text));
    v.push(label);
    match state.data.borrow().transactions {
        Ok(RespType::InProgress) => {
            v.push(new_leaf(to_key(LoadingFrame, "trans loading")).with_attributes(map!("label" => "Getting Transactions...".to_string())));
        }
        Ok(RespType::Done(ref transactions)) => {
            v.push(trans_box(transactions));
        }
        Err(ref e) => {
            v.push(new_leaf(to_key(ErrorPage, "trans error")).with_attributes(map!("label" => e.to_string())));
        }
        _ => {
            let button = new_leaf(GetTransButton)
                .with_attributes(map!("label" => "Get transactions".to_string()))
                .with_callback("clicked", get_trans_cb());
            v.push(button);
        }
    };
    new_node(v, "user_page")
}

fn main_app(state: &AppPtr) -> Component {
    let mut v = Vec::new();
    let signed_in = state.data.borrow().signed_in.clone();
    match signed_in {
        Ok(RespType::InProgress) => {
            v.push(new_leaf(to_key(LoadingFrame, "sign in loading")).with_attributes(map!("label" => "Signing in...".to_string())));
        },
        Ok(RespType::Done(true)) => {
            v.push(user_page(state));
        }
        Err(e) => {
            v.push(new_leaf(to_key(ErrorPage, "sign in error")).with_attributes(map!("label" => e)));
        }
        _ => { v.push(sign_in_page(state)); }
    }
    new_node(v, MainBox)
}

pub fn build_ui(state: AppPtr) {
    let v = vec![main_app(&state)];
    let app_tree = new_node(v, MainWindow);
    app_tree.render_diff(
        state.ui_tree.borrow().as_ref(),
        &(MainWindow, 0),
        &state.widgets,
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