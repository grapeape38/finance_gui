extern crate hyper;
extern crate azul;
use hyper::rt::{self, Future, Stream};

use azul::{prelude::*, widgets::{label::Label, button::Button}, callbacks::{UpdateScreen,TimerCallbackInfo}};

use std::env::args;
use std::time::{Duration};
use std::error::Error;
use serde_json::{Value,json};
use std::sync::{Arc, Mutex};

mod plaid;
use plaid::{ClientHandle, AuthParams, get_access_token};

enum RequestType {
    RequestNone,
    SignIn,
    GetTransactions,
}

impl PartialEq for RequestType {
    fn eq(&self, other: &RequestType) -> bool {
        self == other
    }
}

struct DataModel { 
    last_request_type: RequestType,
    signed_in: bool,
    transactions: Option<Value>,
    auth_params: AuthParams,
    client_handle: Arc<Mutex<Option<ClientHandle>>>
}

impl DataModel {
    fn new() -> DataModel {
        DataModel {
            last_request_type: RequestType::RequestNone,
            signed_in: false,
            auth_params: AuthParams::new().unwrap(),
            client_handle: Arc::new(Mutex::new(None)),
            transactions: None 
        }
    }
}

fn make_call_async<F>(client_handle: Arc<Mutex<Option<ClientHandle>>>, call: F)
    where F: Future<Item=ClientHandle, Error=hyper::Error> + Send + 'static
{
    rt::spawn(rt::lazy(move || {
        call.and_then(move |ch| {
            client_handle.modify(|state| {
                *state = Some(ch);
            }).unwrap();
            Ok(())
        }).map_err(|e| panic!(e))
    }));
}

fn handle_event(state: &mut DataModel, json: Value) {
    match state.last_request_type {
        RequestType::RequestNone => { },
        RequestType::SignIn => {
            state.signed_in = true;
        },
        RequestType::GetTransactions => {
            state.transactions = Some(json);
        }
    }
    state.last_request_type = RequestType::RequestNone;
}

fn poll_handle(event: TimerCallbackInfo<DataModel>) -> (UpdateScreen, TerminateTimer) {
    let mut ret = (Redraw, TerminateTimer::Continue);
    let mut json = json!({});
    if let Some(ref ch) = &mut *event.state.client_handle.try_lock().unwrap() {
        //if let Some(js) = ch.json_result.clone() {
        if ch.json_result.is_some() {
            //json = js.clone();
            /*if event.state.last_request_type == RequestType::SignIn {
                event.state.auth_params = ch.auth_params.clone();  
            }*/
            println!("Finally ready!");
            ret = (DontRedraw, TerminateTimer::Terminate);
        }
        else {
            println!("Not ready yet!");
        }
    } else {
        println!("Not ready yet!");
    }
    if ret.0 == DontRedraw {
        handle_event(event.state, json);
    }
    return ret;
}

fn sign_in(event: CallbackInfo<DataModel>) 
-> UpdateScreen
{
    event.state.data.last_request_type = RequestType::SignIn;
    make_call_async(event.state.data.client_handle.clone(), get_access_token());
    let timer = Timer::new(poll_handle).with_interval(Duration::from_millis(500));
    event.state.add_timer(TimerId::new(), timer);
    Redraw
}

fn get_transactions(event: CallbackInfo<DataModel>) 
-> UpdateScreen
{
    event.state.data.last_request_type = RequestType::GetTransactions;
    let mut ch = ClientHandle::new().unwrap();
    ch.auth_params = event.state.data.auth_params.clone();
    make_call_async(event.state.data.client_handle.clone(), ch.get_transactions());
    let timer = Timer::new(poll_handle).with_interval(Duration::from_millis(1000));
    event.state.add_timer(TimerId::new(), timer);
    Redraw
}
/*fn update_counter(event: CallbackInfo<DataModel>) -> UpdateScreen {
    event.state.data.counter += 1;
    Redraw
}*/

impl DataModel {
    fn main_page(&self) -> Dom<Self> {
        let label = Label::new("You've signed in!").dom();
        let get_trans_button = Button::with_label("Get Transactions").dom()
            .with_callback(On::MouseUp, get_transactions);

        let mut layout = Dom::div()
            .with_child(label)
            .with_child(get_trans_button);
        if let Some(ref trans) = self.transactions {
            let json_str = serde_json::to_string_pretty(&trans).unwrap();
            let label2 = Label::new(json_str).dom();
            layout = layout.with_child(label2);
        }
        layout
    }
}

impl Layout for DataModel {
    fn layout(&self, _: LayoutInfo<Self>) -> Dom<Self> {
        if self.signed_in {
            return self.main_page();
        }
        let button = Button::with_label("Sign in").dom()
            .with_callback(On::MouseUp, sign_in);
        Dom::div()
            .with_child(button)
    }
}

fn azul() {
    let mut app = App::new(DataModel::new(), AppConfig::default()).unwrap();
    let window = app.create_window(WindowCreateOptions::default(), css::native()).unwrap();
    app.run(window).unwrap();
}

fn main() {
    rt::run(rt::lazy(|| {
        azul();
        Ok(())
    }));
}