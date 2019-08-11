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

#[derive(Clone, Debug)]
pub enum RespType<T> {
    None,
    InProgress,
    Done(T)
}

pub type ReqStatus<T> = Result<RespType<T>, String>;

pub fn make_call_async<F, G>(call: F, app: &AppPtr, handle_response_fn: Rc<G>) 
    where F: Future<Item=Value, Error=String> + Send + 'static, G: Fn(Result<Value, String>, AppPtr) + 'static
{
        let app_2 = Rc::clone(app);
        let req_status = Arc::clone(&app_2.async_request);
        let req_err = Arc::clone(&req_status);
        req_status.modify(|st| {
            *st = Ok(RespType::InProgress);
        });
        rt::spawn(rt::lazy(move || {
            call.and_then(move |resp_json| {
                req_status.modify(|st| {
                    *st = Ok(RespType::Done(resp_json));
                }).unwrap();
                Ok(())
            }).map_err(move |e| {
                req_err.modify(|st| {
                    *st = Err(e);
                }).unwrap();
            })
        }));
        timeout_add_seconds(1, move || {
            poll_response(Rc::clone(&app_2), Rc::clone(&handle_response_fn))
        });
}


pub fn poll_response<G>(app: AppPtr, handle_response_fn: Rc<G>) -> Continue
    where G: Fn(Result<Value, String>, AppPtr)
{
    if let Ok(status) = app.async_request.try_lock() {
        match *status {
            Ok(RespType::None) | Ok(RespType::InProgress) => {
                println!("Not finished!");
            }
            Ok(RespType::Done(ref json)) => {
                println!("Got response! {:?}", json);
                handle_response_fn(Ok(json.clone()), Rc::clone(&app));
                return Continue(false);
            }
            Err(ref e) => {
                println!("Error with request: {}", e);
                handle_response_fn(Err(e.to_string()), Rc::clone(&app));
                return Continue(false);
            }
        }
    }
    return Continue(true);
}

pub struct DataModel { 
    pub signed_in: ReqStatus<bool>,
    pub transactions: ReqStatus<Vec<Transaction>>,
    pub auth_params: AuthParams,
}

impl DataModel {
    pub fn new() -> DataModel {
        DataModel {
            signed_in: Ok(RespType::Done(false)),
            auth_params: AuthParams::new().unwrap(),
            transactions: Ok(RespType::None),
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

pub type CallbackFn = Fn(AppPtr);

pub fn sign_in_cb() -> Rc<CallbackFn> {
    Rc::new(|app: AppPtr| {
        app.data.borrow_mut().signed_in = Ok(RespType::InProgress);
        make_call_async(get_access_token(), &app, Rc::new(|json: Result<Value, String>, app2: AppPtr| {
            {
                let mut data = app2.data.borrow_mut();
                data.signed_in = json.as_ref().map(|_| RespType::Done(true)).map_err(|e| e.clone());
                data.auth_params.access_token = json.as_ref().ok().map(|json| json["access_token"].as_str().expect("failed to get public token").to_string());
                data.auth_params.item_id = json.ok().map(|json| json["item_id"].as_str().expect("failed to get item id").to_string());
            }
            build_ui(Rc::clone(&app2));
        }));
        build_ui(Rc::clone(&app));
    })
}

pub fn get_trans_cb() -> Rc<CallbackFn> {
    Rc::new(|app: AppPtr| {
        app.data.borrow_mut().transactions = Ok(RespType::InProgress);
        let mut ch = ClientHandle::new().unwrap();
        ch.auth_params = app.data.borrow().auth_params.clone();
        make_call_async(ch.get_transactions(), &app, Rc::new(|json: Result<Value, String>, app2: AppPtr| {
            {
                let mut data = app2.data.borrow_mut();
                data.transactions = json.map(|mut trans_json|  {
                    let trans_obj = trans_json.get_mut("transactions").expect("no transactions in response").take();
                    RespType::Done(serde_json::from_value(trans_obj).expect("error deserializing transactions"))
                    }).map_err(|e| e.clone());
            }
            build_ui(Rc::clone(&app2));
        }));
        build_ui(Rc::clone(&app));
    })
}