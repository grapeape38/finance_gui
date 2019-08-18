mod gui;
mod plaid;
mod datamodel;
mod component;
mod component2;
mod xml_test;
mod xml_parse;
mod ewidget;
use gui::run_app;

use hyper::rt::{self};

fn main() {
    rt::run(rt::lazy(|| {
        run_app();
        Ok(())
    }));
}