extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::plaid;
use crate::gui;

use hyper::rt::{self, Future, Stream};

use gui::{AppPtr, build_ui};
use gtk::prelude::*;
use serde_json::{Value};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use plaid::*;
use EventType::*;
use std::collections::HashMap;

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum RespType<T> {
    None,
    InProgress,
    Done(T)
}

pub type ReqStatus<T> = Result<RespType<T>, String>;

pub type EventPtr = Arc<Mutex<HashMap<EventType, ReqStatus<Value>>>>;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub enum EventType {
    SignIn,
    GetTrans,
    GetBal
}

impl<T> From<T> for RespType<T> {
    fn from(val: T) -> Self {
        RespType::Done(val)
    }
}

fn add_and_poll_events(events: &Vec<EventType>, app: &AppPtr) -> bool {
    if let Ok(ref mut emap) = app.event_map.lock() {
        if events.iter().any(|e| emap[e] == Ok(RespType::InProgress)) {
            return false;
        }
        events.iter().for_each(|e| {
            emap.insert(e.clone(), Ok(RespType::InProgress));
        });
        let app_2 = Rc::clone(&app);
        timeout_add_seconds(1, move || {
            poll_events(Rc::clone(&app_2))
        });
        return true;
    }
    false
}

fn poll_events(app: AppPtr) -> Continue {
    let mut cont = false;
    let mut rebuild = false;
    if let Ok(ref emap) = app.event_map.lock() {
        let mut finished: Vec<(EventType, Result<Value, String>)> = Vec::new();
        emap.iter().for_each(|(et,rs)| {
            match rs {
                Ok(RespType::None) | Ok(RespType::InProgress) => {
                    println!("Not finished!");
                    cont = true;
                },
                Ok(RespType::Done(ref v)) => {
                    println!("Got response! {:?}", v);
                    finished.push((*et, Ok(v.clone())));
                },
                Err(ref e) => {
                    println!("Error with request: {}", e);
                    finished.push((*et, Err(e.clone())));
                }
            }
        });
        rebuild = finished.len() > 0;
        finished.into_iter().for_each(|(et, rs)| {
            handle_event(et, rs, &app);
        });
    }
    if rebuild {
        build_ui(app);
    }
    Continue(cont)
}

fn handle_event(event: EventType, jres: Result<Value, String>, app: &AppPtr) {
    let mut data = app.data.borrow_mut();
    match event {
        EventType::SignIn => {
            let auth_params: Result<AuthParams, String> = jres.and_then(|json| 
                serde_json::from_value(json.clone()).map_err(|_| "error deserializing auth params".to_string()));
            match auth_params {
                Ok(auth) => {
                    data.signed_in = Ok(RespType::Done(true));
                    data.auth_params.access_token = auth.access_token;
                    data.auth_params.item_id = auth.item_id;
                }
                Err(e) => { data.signed_in = Err(e); }
            };
        },
        EventType::GetTrans => {
             data.transactions = jres.and_then(|trans_json|  {
                serde_json::from_value(trans_json.clone()).map_err(|_| "error deserializing transactions".to_string())
             }).map(|trans_obj: Transactions| {
                trans_obj.into()
             });
        },
        EventType::GetBal => {

        }
    }
}

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
    pub auth_params: AuthParams,
    pub signed_in: ReqStatus<bool>,
    pub transactions: ReqStatus<Transactions>,
    pub balance: ReqStatus<f32>,
}

impl DataModel {
    pub fn new() -> DataModel {
        DataModel {
            auth_params: AuthParams::new().unwrap(),
            signed_in: Ok(RespType::None),
            transactions: Ok(RespType::None),
            balance: Ok(RespType::None)
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
        add_and_poll_events(&vec![SignIn, GetBal, GetTrans], &app);
        let event_map = Arc::clone(&app.event_map);
        let emap2 = Arc::clone(&app.event_map);
        rt::spawn(get_access_token().then(move |res| {
                event_map.modify(|emap| {
                    emap.insert(SignIn, res.as_ref().map(|(_, json)| json.clone().into()).map_err(|e| e.to_string()));
                });
                res.map(|(ch, _)| ch)
            }).and_then(|ch| {
                ch.get_transactions()
            }).then(move |res| {
                emap2.modify(|emap| {
                    emap.insert(GetTrans, res.map(|json| json.into()));
                });
                Ok(())
            })
        );
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