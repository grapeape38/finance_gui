extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::plaid;
use crate::gui;

use hyper::rt::{self, Future, Stream};

use gui::{AppPtr, build_ui};
use gio::prelude::*;
use gtk::{prelude::*, timeout_add_seconds};
use serde_json::{Value};
use std::rc::Rc;
use std::error::Error;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use plaid::{ClientHandle, AuthParams, get_access_token};

#[derive(Debug, Clone)]
pub enum RequestType {
    None,
    SignIn,
    GetTransactions,
}

#[derive(Debug, Clone)]
pub enum RequestStatus {
    None,
    InProgress,
    Ok(Value),
    Err(String)
}

pub struct DataModel { 
    pub signed_in: bool,
    transactions: Option<Value>,
    auth_params: AuthParams,
    req_status: Arc<Mutex<RequestStatus>>
}

impl DataModel {
    pub fn new() -> DataModel {
        DataModel {
            signed_in: false,
            auth_params: AuthParams::new().unwrap(),
            transactions: None,
            req_status: Arc::new(Mutex::new(RequestStatus::None)) 
        }
    }
}

pub type DataPtr = Rc<RefCell<DataModel>>;

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

fn make_call_async<F>(req_status: Arc<Mutex<RequestStatus>>, call: F)
    where F: Future<Item=Value, Error=String> + Send + 'static
{
    let req_err = Arc::clone(&req_status);
    //let call_send = call.map_err(|e| e.to_string());
    rt::spawn(rt::lazy(move || {
        call.and_then(move |resp_json| {
            req_status.modify(|st| {
                *st = RequestStatus::Ok(resp_json);
            }).unwrap();
            Ok(())
        }).map_err(move |e| {
            req_err.modify(|st| {
                *st = RequestStatus::Err(e);
            }).unwrap();
        })
    }));
}

fn handle_response_ok(state: AppPtr, req_type: RequestType, json: Value) {
    let mut state = state.borrow_mut();
    match req_type {
        RequestType::None=> { },
        RequestType::SignIn => {
            state.data.signed_in = true;
            state.data.auth_params.access_token = Some(json["access_token"].as_str().expect("failed to get public token").to_string());
            state.data.auth_params.item_id = Some(json["item_id"].as_str().expect("failed to get item id").to_string());
        },
        RequestType::GetTransactions => {
            state.data.transactions = Some(json);
        }
    }
}

pub fn poll_response(state: AppPtr, req_type: RequestType) -> Continue {
    let req_clone = state.borrow().async_request.modify_clone(|st| {
        st.clone()
    });
    if let Some(req_clone) = req_clone {
        match req_clone {
            RequestStatus::None | RequestStatus::InProgress => {
                println!("Not finished!");
            }
            RequestStatus::Ok(json) => {
                println!("Got response!");
                handle_response_ok(state, req_type, json);
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

pub fn sign_in(state: AppPtr) {
    state.borrow_mut().async_request.modify(|req| {
        *req = RequestStatus::InProgress;
    });
    make_call_async(Arc::clone(&state.borrow().async_request), get_access_token());
    timeout_add_seconds(1, move || {
        poll_response(Rc::clone(&state), RequestType::SignIn)
    });
}