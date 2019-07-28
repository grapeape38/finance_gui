extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::plaid;
use crate::gui;

use hyper::rt::{self, Future, Stream};

use gui::AppState;
use gio::prelude::*;
use gtk::{prelude::*, timeout_add_seconds};
use serde_json::{Value};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
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

pub struct DataModel { 
    pub signed_in: bool,
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

pub type DataPtr = Rc<RefCell<DataModel>>;
type AppPtr<'a> = Rc<RefCell<AppState<'a>>>;

pub fn create_model() -> DataPtr {
    Rc::new(RefCell::new(DataModel::new()))
}


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

pub fn sign_in(state: DataPtr) {
    state.borrow_mut().request.modify(|req| {
        req.req_type = RequestType::SignIn;
        req.req_status = RequestStatus::InProgress;
    });
    make_call_async(Arc::clone(&state.borrow().request), get_access_token());
    timeout_add_seconds(1, move || {
        poll_response(Rc::clone(&state))
    });
}