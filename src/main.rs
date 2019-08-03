mod gui;
mod plaid;
mod datamodel;
mod component;
use gui::run_app;

use hyper::rt::{self};

fn main() {
    rt::run(rt::lazy(|| {
        run_app();
        Ok(())
    }));
}