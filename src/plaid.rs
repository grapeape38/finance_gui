extern crate hyper;

use std::io::{self, Write};
use hyper::Client;
use hyper::rt::{self, Future, Stream};

pub fn makeRequest() {
    rt::run(rt::lazy(|| {
        let client = Client::new();

        let uri = "http://httpbin.org/ip".parse().unwrap();

        client
            .get(uri)
            .and_then(|res| {
                println!("Response: {}", res.status());
                res
                    .into_body()
                    // Body is a stream, so as each chunk arrives...
                    .for_each(|chunk| {
                        io::stdout()
                            .write_all(&chunk)
                            .map_err(|e| {
                                panic!("example expects stdout is open, error={}", e)
                            })
                    })
            })
            .map_err(|err| {
                println!("Error: {}", err);
            })
        }));
}