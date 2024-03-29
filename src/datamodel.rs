extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::plaid;
use crate::gui;

use hyper::rt::{self, Future, Stream};

use gui::{AppPtr, build_ui};
use gtk::prelude::*;
use serde::{Deserialize};
use serde_json::{Value};
use std::rc::Rc;
use std::time::Duration;
use tokio_timer::{sleep};
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

trait ToState<T> where for<'de> T: Deserialize<'de>
{
    fn to_state(self) -> ReqStatus<T>;
}

impl<T> ToState<T> for Result<Value, String> where for<'de> T: Deserialize<'de>
{
    fn to_state(self) -> ReqStatus<T> {
        self.and_then(|json|  {
            serde_json::from_value(json).map_err(|e| e.to_string())
        }).map(|obj: T| obj.into())
    }
}

impl<T> ToState<T> for ReqStatus<Value> where for<'de> T: Deserialize<'de> {
    fn to_state(self) -> ReqStatus<T> {
        match self {
            Ok(RespType::Done(done)) => Ok(done).to_state(),
            Err(e) => Err(e),
            Ok(RespType::InProgress) => Ok(RespType::InProgress),
            Ok(RespType::None) => Ok(RespType::None),
        }
    }
}

fn add_and_poll_events(events: &Vec<EventType>, app: &AppPtr) -> bool {
    if let Ok(ref mut emap) = app.event_map.lock() {
        if events.iter().any(|e| emap.get(e) == Some(&Ok(RespType::InProgress))) {
            return false;
        }
        events.iter().for_each(|e| {
            emap.insert(*e, Ok(RespType::InProgress));
            app.data.borrow_mut().handle_event(*e, Ok(RespType::InProgress));
        });
        let app_2 = Rc::clone(&app);
        timeout_add(500, move || {
            poll_events(Rc::clone(&app_2))
        });
        return true;
    }
    false
}

fn poll_events(app: AppPtr) -> Continue {
    let mut cont = false;
    let mut rebuild = false;
    if let Ok(ref mut emap) = app.event_map.lock() {
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
            emap.remove(&et);
            app.data.borrow_mut().handle_event(et, rs.map(|rt| rt.into()));
        });
    }
    if rebuild {
        println!("Rebuilding!");
        build_ui(app);
    }
    Continue(cont)
}

pub struct DataModel { 
    pub auth_params: ReqStatus<AuthParams>,
    pub transactions: ReqStatus<Transactions>,
    pub accounts: ReqStatus<Accounts>,
}

impl DataModel {
    pub fn new() -> DataModel {
        DataModel {
            auth_params: Ok(RespType::None),
            transactions: Ok(RespType::None),
            accounts: Ok(RespType::None)
        }
    }
    fn handle_event(&mut self, et: EventType, rs: ReqStatus<Value>) {
        match et {
            SignIn => { self.auth_params = rs.to_state(); },
            GetTrans => { self.transactions = rs.to_state(); },
            GetBal => { self.accounts = rs.to_state(); }
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

trait UpdateEventMap {
    fn update_event_ref(&self, event: EventType, json_res: Result<&Value, &String>);
    fn update_event(&self, event: EventType, json_res: &Result<Value, String>) {
        self.update_event_ref(event, json_res.as_ref());
    }
}

impl UpdateEventMap for EventPtr {
    fn update_event_ref(&self, event: EventType, json_res: Result<&Value, &String>) {
        self.modify(|emap| {
            emap.insert(event, json_res.map(|json| json.clone().into()).map_err(|e| e.clone()));
        });
    }
}

pub type CallbackFn = Fn(AppPtr);

pub fn sign_in_cb() -> Rc<CallbackFn> {
    Rc::new(|app: AppPtr| {
        if add_and_poll_events(&vec![SignIn, GetBal, GetTrans], &app) {
            let event_map = Arc::clone(&app.event_map);
            let emap2 = Arc::clone(&app.event_map);
            rt::spawn(get_access_token().then(move |res| {
                    let json = res.as_ref().map(|r| &r.1);
                    event_map.update_event_ref(SignIn, json);
                    sleep(Duration::from_millis(1500)).map_err(|e| e.to_string())
                        .and_then(|_| res.map(|r| r.0))
                }).and_then(|ch| {
                    ch.get_balance().join(ch.get_transactions())
                }).then(move |tup| {
                    let bal = tup.as_ref().map(|t| &t.0);
                    let trans = tup.as_ref().map(|t| &t.1);
                    emap2.update_event_ref(GetBal, bal);
                    emap2.update_event_ref(GetTrans, trans);
                    Ok(())
                })
            );
            build_ui(Rc::clone(&app));
        }
    })
}

pub fn get_trans_cb() -> Rc<CallbackFn> {
    Rc::new(|app: AppPtr| {
        if add_and_poll_events(&vec![GetTrans], &app) {
            let event_map = Arc::clone(&app.event_map);
            let mut ch = ClientHandle::new().unwrap(); 
            let auth = app.data.borrow().auth_params.clone();
            if let Ok(RespType::Done(auth)) = auth {
                ch.auth_params = auth; 
                rt::spawn(ch.get_transactions().then(move |res| {
                    event_map.update_event(GetTrans, &res);
                    Ok(())
                }));
            }
        }
    })
}