extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::plaid;
use crate::gui;

use hyper::rt::{self, Future, Stream};

use gui::{AppPtr, build_ui};
use gio::prelude::*;
use gtk::prelude::*;
use serde_json::{Value};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use plaid::*;

#[derive(Debug, Clone)]
pub enum RequestType {
    NoReq,
    SignIn,
    GetTransactions,
}

#[derive(Debug, Clone)]
pub enum RequestStatus {
    NoReq,
    InProgress,
    Ok(Value),
    Err(String)
}

pub struct AsyncCallback<F> 
where F: Future<Item=Value, Error=String> + Send + 'static
{
    pub req_type: RequestType,
    pub fut: Box<Fn(&DataModel) -> F>
}

impl<F> AsyncCallback<F> 
    where F: Future<Item=Value, Error=String> + Send + 'static
{
    pub fn make_call_async(&self, app: &AppPtr) {
        let app_2 = Rc::clone(app);
        let call = (self.fut)(&app_2.data.borrow());
        let req_status = Arc::clone(&app_2.async_request);
        let req_err = Arc::clone(&req_status);
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
}

pub struct DataModel { 
    pub signed_in: bool,
    transactions: Option<Value>,
    auth_params: AuthParams,
}

impl DataModel {
    pub fn new() -> DataModel {
        DataModel {
            signed_in: false,
            auth_params: AuthParams::new().unwrap(),
            transactions: None,
        }
    }
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

fn handle_response_ok(state: AppPtr, req_type: &RequestType, json: Value) {
    let mut state_changed = true;
    {
        let mut data = state.data.borrow_mut();
        match req_type {
            RequestType::NoReq => { state_changed = false; },
            RequestType::SignIn => {
                data.signed_in = true;
                data.auth_params.access_token = Some(json["access_token"].as_str().expect("failed to get public token").to_string());
                data.auth_params.item_id = Some(json["item_id"].as_str().expect("failed to get item id").to_string());
            },
            RequestType::GetTransactions => {
                data.transactions = Some(json);
            }
        }
    }
    if state_changed {
        build_ui(state);
    }
}

pub fn poll_response(state: AppPtr, req_type: &RequestType) -> Continue {
    let req_clone = state.async_request.modify_clone(|st| {
        st.clone()
    });
    if let Some(req_clone) = req_clone {
        match req_clone {
            RequestStatus::NoReq | RequestStatus::InProgress => {
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

pub fn sign_in_cb() -> Rc<AsyncCallback<impl Future<Item=Value, Error=String>>>
{
    Rc::new(AsyncCallback {
        req_type: RequestType::SignIn,
        fut: Box::new(|_| { get_access_token() })
    })
}

pub fn get_transactions_cb() -> Rc<AsyncCallback<impl Future<Item=Value, Error=String>>> {
    Rc::new(AsyncCallback {
        req_type: RequestType::GetTransactions,
        fut: Box::new(|data| {
            let auth_params = data.auth_params.clone();
            let mut ch = ClientHandle::new().unwrap();
            ch.auth_params = auth_params;
            ch.get_transactions()
        })
    })
}
