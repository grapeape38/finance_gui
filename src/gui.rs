extern crate gio;
extern crate gtk;

extern crate hyper;

use crate::datamodel::*;
use crate::component::*;
use crate::plaid::{AuthParams, Transaction, Transactions, Account, Accounts};
use crate::ewidget::{*, EWidget::*};

use gio::prelude::*;
use gtk::{prelude::*, Widget};
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
    pub data: RefCell<DataModel>,
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

        Rc::new(AppState {
            data: RefCell::new(DataModel::new()),
            event_map: Arc::new(Mutex::new(HashMap::new())),
            ui_tree: RefCell::new(None),
            widgets
        })
    }
}

fn label_frame(text: &str, id: &str) -> Component {
    let label = new_leaf((SomeLabel, id)).with_attributes(map!("text" => text.to_string()));
    new_node(vec![label], (LabelFrame, id))
}

fn loading_comp<T, F, G>(state: &AppPtr, value: ReqStatus<T>, init: F, done: G, key_s: &'static str, loading_msg: &'static str) -> Component
 where F: Fn(&AppPtr) -> Component, G: Fn(&AppPtr, &T) -> Component {
     let err_s = format!("{}-error", key_s);
     match value {
        Ok(RespType::InProgress) => label_frame(loading_msg, key_s),
        Ok(RespType::Done(ref val)) => done(state, val),
        Err(ref e) => label_frame(e.as_str(), &err_s),
        _ => init(state)
     }
}

fn trans_row(trans: &Transaction) -> Component {
    let amt = format!("{}", trans.amount);
    let entries = vec![
        //&trans.account_id,
        &amt,
        &trans.date,
        &trans.name,
        //&trans.transaction_id,
        &trans.transaction_type
    ];
    let mut i = 0;
    let rowvec = entries.into_iter().map(|entry| {
        i += 1;
        label_frame(entry, &format!("{}-{}", trans.transaction_id, i))
    }).collect();
    new_node(rowvec, (TransRow, &trans.transaction_id)).with_attributes(map!("orientation" => "horizontal".to_string()))
}

fn trans_box(tr: &Transactions) -> Component {
    let mut v = Vec::new();
    v.push(label_frame("Transactions: ", "trans_frame"));
    v.extend(tr.transactions.iter().map(|t| trans_row(t)));
    new_node(v, TransBox)
}

fn acct_box(acct: &Account) -> Component {
    let labels = vec![
        format!("Name: {}", acct.name), 
        format!("Available Balance: {}", acct.balances.available.unwrap_or(0.)), 
        format!("Current Balance: {}", acct.balances.current)];
    let mut i = 0;
    let v = labels.iter().map(|l| {
        i += 1;
        label_frame(l, &format!("{}-{}", acct.account_id, i))
    }).collect();
    new_node(v, (AccountBox, &acct.name))
}

fn accts(accts: &Accounts) -> Component {
    let mut v = Vec::new();
    v.push(label_frame("Accounts: ", "accounts_frame"));
    v.extend(accts.accounts.iter().map(|acc| acct_box(acc)));
    new_node(v, (AccountBox, "main")).with_attributes(map!("orientation" => "horizontal".to_string()))
}

fn sign_in_page() -> Component {
    new_leaf(SignInButton)
        .with_attributes(map!("label" => "Sign in!".to_string()))
        .with_callback("clicked", sign_in_cb())
}

/*let trans_button = |_: &AppPtr| {
    new_leaf(GetTransButton)
        .with_attributes(map!("label" => "Get transactions".to_string()))
        .with_callback("clicked", get_trans_cb())
};*/

fn user_page(state: &AppPtr, auth: &AuthParams) -> Component {
    let mut v = Vec::new();
    let label_text = format!("You are signed in! Your access token is: {}", auth.access_token.as_ref()
        .unwrap_or(&("".to_string())));
    v.push(label_frame(&label_text, "access token label"));

    let transactions = state.data.borrow().transactions.clone();
    let t_none = |_: &AppPtr| Component::empty("transempty");
    let tbox = |_: &AppPtr, t: &Transactions| trans_box(t);

    let accounts = state.data.borrow().accounts.clone();
    let accts_none = |_: &AppPtr| Component::empty("balnone");
    let acctsbox = |_: &AppPtr, a: &Accounts| accts(a);

    v.push(loading_comp(state, accounts, accts_none, acctsbox, "balances", "Getting Balances..."));
    v.push(loading_comp(state, transactions, t_none, tbox, "transactions", "Getting Transactions..."));
    new_node(v, "user_page")
}

fn main_app(state: &AppPtr) -> Component {
    let signed_in = state.data.borrow().auth_params.clone();
    let spage = |_: &AppPtr| sign_in_page();
    let upage = |state: &AppPtr, auth: &AuthParams| user_page(state, auth);
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