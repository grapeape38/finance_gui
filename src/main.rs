/*mod gui;
mod plaid;
mod datamodel;
mod component;
mod ewidget;
use gui::run_app;
use hyper::rt::{self};*/

mod component2;
mod xml_test;
mod xml_parse;
use xml_test::{test_main};


fn main() {
    /*rt::run(rt::lazy(|| {
        run_app();
        Ok(())
    }));*/
    test_main();
}