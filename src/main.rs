mod gui;
mod plaid;
mod datamodel;
mod component;
use gui::run_app;

use hyper::rt::{self};
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




fn main() {
    rt::run(rt::lazy(|| {
        run_app();
        Ok(())
    }));
}