extern crate gio;
extern crate gtk;

extern crate hyper;

use crate::datamodel::*;
use crate::component::*;
use crate::plaid::{Transaction, Transactions};
use crate::ewidget::{*, EWidget::*};

use gio::prelude::*;
use gtk::{prelude::*, Widget};
use std::env::args;
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

pub struct AppState {
    pub data: RefCell<DataModel>,
    pub async_request: Arc<Mutex<ReqStatus<Value>>>,
    pub event_map: EventPtr,
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
        widgets.get_mut(&MainWindow).unwrap().set(window.upcast::<Widget>(), "".to_string());
        let app_state = AppState {
            data: RefCell::new(DataModel::new()),
            async_request: Arc::new(Mutex::new(Ok(RespType::None))),
            event_map: Arc::new(Mutex::new(HashMap::new())),
            ui_tree: RefCell::new(None),
            widgets
        };
        Rc::new(app_state)
    }
}

fn loading_comp<T, F, G>(state: &AppPtr, value: ReqStatus<T>, init: F, done: G, key_s: &'static str, loading_msg: &'static str) -> Component
 where F: Fn(&AppPtr) -> Component, G: Fn(&AppPtr, &T) -> Component {
     match value {
        Ok(RespType::InProgress) => 
            new_leaf((LoadingFrame, key_s)).with_attributes(map!("label" => loading_msg.to_string())),
        Ok(RespType::Done(ref val)) => done(state, val),
        Err(ref e) => 
            new_leaf((ErrorPage, key_s)).with_attributes(map!("label" => e.to_string())),
        _ => init(state)
     }
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
        let mut key = (TransColLabel, format!("{}-{}", trans.transaction_id, i));
        let label = new_leaf(key).with_attributes(map!("text" => entry.clone()));
        key = (TransColBin, format!("{}-{}-bin", trans.transaction_id, i));
        i += 1;
        new_node(vec![label], key)
    }).collect();
    new_node(rowvec, (TransRow, &trans.transaction_id)).with_attributes(map!("orientation" => "horizontal".to_string()))
}

fn trans_box(tr: &Transactions) -> Component {
    let rows = tr.transactions.iter().map(|t| {
        trans_row(t)
    }).collect();
    new_node(rows, TransBox)
}

fn sign_in_page() -> Component {
    new_leaf(SignInButton)
        .with_attributes(map!("label" => "Sign in!".to_string()))
        .with_callback("clicked", sign_in_cb())
}

fn user_page(state: &AppPtr) -> Component {
    let mut v = Vec::new();
    let label_text = format!("You are signed in! Your access token is: {}", state.data.borrow().auth_params.access_token.as_ref()
        .unwrap_or(&("".to_string())));
    let label = new_leaf(SignedInFrame)
        .with_attributes(map!("label" => label_text));
    v.push(label);

    let transactions = state.data.borrow().transactions.clone();
    let trans_button = |_: &AppPtr| {
        new_leaf(GetTransButton)
            .with_attributes(map!("label" => "Get transactions".to_string()))
            .with_callback("clicked", get_trans_cb())
    };
    let tbox = |_: &AppPtr, t: &Transactions| {
        trans_box(t)
    };
    v.push(loading_comp(state, transactions, trans_button, tbox, "transactions", "Getting Transactions..."));
    new_node(v, "user_page")
}

fn main_app(state: &AppPtr) -> Component {
    let signed_in = state.data.borrow().signed_in.clone();
    let spage = |_: &AppPtr| { sign_in_page() };
    let upage = |state: &AppPtr, _: &bool| { user_page(state) };
    let v = vec![loading_comp(state, signed_in, spage, upage, "sign in", "Signing in...")];
    new_node(v, MainBox)
}

pub fn build_ui(state: AppPtr) {
    let v = vec![main_app(&state)];
    let app_tree = new_node(v, MainWindow);
    app_tree.render_diff(
        state.ui_tree.borrow().as_ref(),
        &(MainWindow, "".to_string()),
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