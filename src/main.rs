extern crate gio;
extern crate gtk;

extern crate hyper;
use hyper::rt::{self, Future, Stream};

use gio::prelude::*;
use gtk::{prelude::*, timeout_add_seconds};
use std::env::args;
use std::time::{Duration};
use std::error::Error;
use serde_json::{Value,json};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
mod plaid;
use plaid::{ClientHandle, AuthParams, get_access_token};

#[derive(Debug, Clone)]
enum RequestType {
    None,
    SignIn,
    GetTransactions,
}

#[derive(Debug, Clone)]
enum RequestStatus {
    None,
    InProgress,
    Ok(Value),
    Err(String)
}

#[derive(Debug, Clone)]
struct Request {
    req_type: RequestType,
    req_status: RequestStatus 
}

impl Request {
    fn arc_none() -> Arc<Mutex<Request>> {
        Arc::new(Mutex::new(Request { 
            req_type: RequestType::None,
            req_status: RequestStatus::None
        }))
    }
}

struct DataModel { 
    signed_in: bool,
    transactions: Option<Value>,
    auth_params: AuthParams,
    request: Arc<Mutex<Request>>
}

impl DataModel {
    fn new() -> DataModel {
        DataModel {
            signed_in: false,
            auth_params: AuthParams::new().unwrap(),
            transactions: None,
            request: Request::arc_none()
        }
    }
}

type DataPtr = Rc<RefCell<DataModel>>;

trait Modify<T> {
    fn modify<F>(&self, closure: F) -> Option<()> where F: FnOnce(&mut T);

    fn modify_clone<F, S>(&self, closure: F) -> Option<S> where F: FnOnce(&mut T) -> S {
        let mut initial: Option<S> = None;
        self.modify(|lock| initial = Some(closure(lock)))?;
        initial
    }
}

impl<T> Modify<T> for Arc<Mutex<T>> {
    fn modify<F>(&self, closure: F) -> Option<()> where F: FnOnce(&mut T) {
        let mut lock = self.lock().ok()?;
        closure(&mut *lock);
        Some(())
    }
}

fn make_call_async<F>(request: Arc<Mutex<Request>>, call: F)
    where F: Future<Item=Value, Error=hyper::Error> + Send + 'static
{
    let req_err = Arc::clone(&request);
    rt::spawn(rt::lazy(move || {
        call.and_then(move |resp_json| {
            request.modify(|st| {
                st.req_status = RequestStatus::Ok(resp_json);
            }).unwrap();
            Ok(())
        }).map_err(move |e| {
            req_err.modify(|st| {
                st.req_status = RequestStatus::Err(e.to_string());
            }).unwrap();
        })
    }));
}

fn handle_response_ok(state: DataPtr, req_type: RequestType, json: Value) {
    let mut state = state.borrow_mut();
    match req_type {
        RequestType::None=> { },
        RequestType::SignIn => {
            state.signed_in = true;
            state.auth_params.access_token = Some(json["access_token"].as_str().expect("failed to get public token").to_string());
            state.auth_params.item_id = Some(json["item_id"].as_str().expect("failed to get item id").to_string());
        },
        RequestType::GetTransactions => {
            state.transactions = Some(json);
        }
    }
}

fn poll_response(state: DataPtr) -> Continue {
    let req_clone = state.borrow().request.modify_clone(|st| {
        st.clone()
    });
    if let Some(req_clone) = req_clone {
        match req_clone.req_status {
            RequestStatus::None | RequestStatus::InProgress => {
                println!("Not finished!");
            }
            RequestStatus::Ok(json) => {
                println!("Got response!");
                handle_response_ok(state, req_clone.req_type, json);
                return Continue(false);
            }
            RequestStatus::Err(e) => {
                println!("Error with request: {}", e);
                return Continue(false);
            }
        }
    }
    return Continue(true);
}

fn sign_in(state: DataPtr) {
    state.borrow_mut().request.modify(|req| {
        req.req_type = RequestType::SignIn;
        req.req_status = RequestStatus::InProgress;
    });
    make_call_async(Arc::clone(&state.borrow().request), get_access_token());
    timeout_add_seconds(1, move || {
        poll_response(Rc::clone(&state))
    });
}

/*fn poll_handle(event: TimerCallbackInfo<DataModel>) -> (UpdateScreen, TerminateTimer) {
    let mut ret = (DontRedraw, TerminateTimer::Continue);
    if let Ok(guard) = event.state.async_data.try_lock() {
        event.state.sync_data = Some(guard.clone());
        ret = match guard.req_status {
            RequestStatus::Done => (Redraw, TerminateTimer::Terminate),
            _ => ret
        };
    }
    if ret.0 == DontRedraw {
        println!("Not finished");
    }
    else {
        println!("Finished!");
    }
    return ret;
}

fn sign_in(event: CallbackInfo<DataModel>)
-> UpdateScreen
{
    event.state.data.async_data.modify(|st| {
        st.req_type = RequestType::SignIn;
        st.req_status = RequestStatus::Loading;
    }).unwrap();
    make_call_async(Arc::clone(&event.state.data.async_data), get_access_token());
    let timer = Timer::new(poll_handle).with_interval(Duration::from_millis(1000));
    event.state.add_timer(TimerId::new(), timer);
    Redraw
}

fn get_transactions(event: CallbackInfo<DataModel>) 
-> UpdateScreen
{
    event.state.data.async_data.modify(|state| {
        state.req_type = RequestType::GetTransactions;
    });
    let mut ch = ClientHandle::new().unwrap();
    ch.auth_params = event.state.data.async_data.lock().unwrap().auth_params.clone();
    make_call_async(Arc::clone(&event.state.data.async_data), ch.get_transactions());
    let timer = Timer::new(poll_handle).with_interval(Duration::from_millis(500));
    event.state.add_timer(TimerId::new(), timer);
    Redraw
}

impl DataModel {
    fn main_page(&self) -> Dom<Self> {
        let label = Label::new("You've signed in!").dom()
            .with_class("label");
        let get_trans_button = Button::with_label("Get Transactions").dom()
            .with_class("button")
            .with_callback(On::MouseUp, get_transactions);

        let layout = Dom::div()
            .with_child(label)
            .with_child(get_trans_button);
        if let Some(ref data) = self.sync_data {
            if let Some(ref trans) = data.transactions {
                let json_str = serde_json::to_string_pretty(&trans).unwrap();
                let label2 = Label::new(json_str).dom().with_class("label");
                return layout.with_child(label2)
            }
        }
        layout
    }
}

impl Layout for DataModel {
    fn layout(&self, _: LayoutInfo<Self>) -> Dom<Self> {
        if let Some(ref data) = self.sync_data {
            if data.signed_in {
                return self.main_page();
            }
        }

        let button = Button::with_label("Sign in").dom()
            .with_class("button")
            .with_callback(On::MouseUp, sign_in);
        let loading_label = Label::new("Loading...").dom();
        let layout = Dom::div();
        if let Ok(guard) = self.async_data.try_lock() {
            match guard.req_status {
                RequestStatus::Loading => return layout.with_child(loading_label),
                RequestStatus::Done => {} 
            };
        }
        layout.with_child(button)
    }
}
*/

fn build_ui(application: &gtk::Application, data: DataPtr) {
    let window = gtk::ApplicationWindow::new(application);
    window.set_title("First GTK+ Program");
    window.set_border_width(10);
    window.set_position(gtk::WindowPosition::Center);
    window.set_default_size(350, 70);
    let data2 = Rc::clone(&data);
    if data.borrow().signed_in {
        let button = gtk::Button::new_with_label("Sign in");
        button.connect_clicked(move |_| {
            println!("Clicked!");
            sign_in(Rc::clone(&data));
        });
        window.add(&button);
    }
    else {
        let label = gtk::Label::new_with_mnemonic(Some("_You are signed in!"));

    }
    window.show_all();
}

fn run_app() {
    let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");

    let data_ptr = Rc::new(RefCell::new(DataModel::new()));
    application.connect_activate(move |app| {
        build_ui(app, Rc::clone(&data_ptr));
    });

    application.run(&args().collect::<Vec<_>>());
}


fn main() {
    rt::run(rt::lazy(|| {
        run_app();
        Ok(())
    }));
}