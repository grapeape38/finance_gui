extern crate hyper;
extern crate rand;

use rand::{Rng};
use std::sync::{Arc, Mutex};
//use std::borrow::{Borrow, BorrowMut};
use std::io::{self, Write};
use hyper::{Client, Method, Body, Request, Response};
use hyper::client::{HttpConnector};
use hyper::header::{HeaderValue, HeaderMap};
use hyper::rt::{self, Future, Stream};
use hyper_tls::HttpsConnector;
use std::env;
use std::error::Error;
use serde::{Serialize, Deserialize, de};
use serde_json::{json, Value, Map};

const LINK_VERSION: &'static str = "2.0.264";

const CREDENTIALS: Credentials =
    Credentials {
        username: "user_good",
        password: "pass_good" 
    };

#[derive(Debug, Serialize, Clone)]
struct Params {
    link_persistent_id: String,
    link_open_id: String,
    public_key: String,
    #[serde(skip_serializing_if="Option::is_none")]
    country_codes: Option<Vec<String>>,
    #[serde(skip_serializing_if="Option::is_none")]
    link_version: Option<&'static str>,
    initial_products: Vec<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    link_session_id: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    options: Option<Map<String, Value>>,
    #[serde(skip_serializing_if="Option::is_none")]
    institution_id: Option<&'static str>,
    #[serde(skip_serializing_if="Option::is_none")]
    display_language: Option<&'static str>,
    #[serde(skip_serializing_if="Option::is_none")]
    flexible_input_responses: Option<Value>,
    #[serde(skip_serializing_if="Option::is_none")]
    credentials: Option<Credentials>
}

#[derive(Debug, Serialize, Clone)]
struct AuthParams {
    access_token: Option<String>,
    secret: String,
    client_id: String,
}

fn gen_random_id() -> String {
    let mut rng = rand::thread_rng();
    let rand_ints = (0..16).map(|_| rng.gen_range(0,256));
    let hex_bytes : Vec<String> = (256..512).map(|x| format!("{:x}", x)).collect();
    let m : Vec<&str> = rand_ints.map(|x| &hex_bytes[x][1..]).collect();
    return format!("{}{}{}{}-{}{}-{}{}-{}{}-{}{}{}{}{}{}", m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10], m[11], m[12],m[13],m[14],m[15]); 
}

impl Params {
    fn new() -> Result<Params, Box<Error>>  {
        let public_key = env::var("PLAID_PUBLIC_KEY")?;
        let country_codes = env::var("PLAID_COUNTRY_CODES")?;
        let c_codes: Vec<String> = country_codes.split(',').map(|s| s.to_string()).collect();
        let mut initial_products: Vec<String> = Vec::new();
        initial_products.push("transactions".to_string());
        Ok(Params {
            link_open_id: gen_random_id(), 
            link_persistent_id: gen_random_id(), 
            public_key: public_key,
            country_codes: Some(c_codes),
            initial_products: initial_products,
            link_version: Some(LINK_VERSION),
            link_session_id: None,
            display_language: Some("en"),
            flexible_input_responses: Some(Value::Null),
            institution_id: Some("ins_1"),
            options: Some(Map::new()),
            credentials: Some(CREDENTIALS)
        })
    }
}

impl AuthParams {
    fn new() -> Result<AuthParams, Box<Error>> {
        let client_id = env::var("PLAID_CLIENT_ID")?;
        let secret = env::var("PLAID_SECRET")?;
        Ok(AuthParams {
            access_token: None,
            secret: secret,
            client_id: client_id
        })
    }
}

#[derive(Debug, Serialize, Clone)]
struct Credentials {
    username: &'static str,
    password: &'static str,
}

const LINK_HEADERS: &'static [(&'static str, &'static str)] =
   &[ 
        ("Content-Type", "application/json"),
        ("User-Agent",
         "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:68.0) Gecko/20100101 Firefox/68.0"),
        ("Plaid-Link-Version", LINK_VERSION)
   ];

#[derive(Debug)]
struct ClientHandle {
    headers: HeaderMap,
    params: Params,
    client: Client<HttpsConnector<HttpConnector>>
}

impl ClientHandle {
    fn new() -> Result<ClientHandle, Box<Error>> {
        let mut headers = HeaderMap::new();
        LINK_HEADERS.iter().for_each(|h| {
            headers.insert(h.0, HeaderValue::from_static(h.1));
        });
        let https = HttpsConnector::new(4).unwrap();
        let client = Client::builder().build::<_, hyper::Body>(https);
        Ok(ClientHandle {
            params: Params::new()?,
            headers: headers,
            client: client
        })
    }

    fn post_json(&self, json: &str, uri: &str) -> impl Future<Item=Value, Error=hyper::Error> {
        let uri: hyper::Uri = uri.parse().unwrap();
        let mut req = Request::new(Body::from(json.to_string()));
        *req.method_mut() = Method::POST;
        *req.uri_mut() = uri.clone();
        *req.headers_mut() = self.headers.clone();
        self.client.request(req).and_then(|res| {
                println!("Response: {}", res.status());
                res.into_body().concat2()
            }).and_then(|body| {
                let resp_json: Value = serde_json::from_slice(&body).expect("json parsing error");
                Ok(resp_json)
            })
    }

    fn get_session_id(&self) -> impl Future<Item=String, Error=hyper::Error> {
        let mut params = self.params.clone();
        params.link_session_id = None;
        params.display_language = None;
        params.flexible_input_responses = None;
        params.institution_id = None;
        params.options = None;
        params.credentials = None;
        let json = serde_json::to_string_pretty(&params).unwrap();
        println!("Getting session id, json: {}", json);
        let url = "https://sandbox.plaid.com/link/client/get";
        self.post_json(&json, url).and_then(|resp_json| {
            Ok(resp_json["link_session_id"].as_str().expect("failed to get session id").to_string())
        })
    }

    fn get_public_token(&self, sess_id: &str) -> impl Future<Item=String, Error=hyper::Error> {
        let mut params = self.params.clone();
        params.link_session_id = Some(sess_id.to_string());
        params.link_version = None;
        params.country_codes = None;
        let json = serde_json::to_string_pretty(&params).unwrap();
        println!("Getting public token, json: {}", json);
        let url = "https://sandbox.plaid.com/link/item/create";
        self.post_json(&json, url).and_then(|resp_json| {
            Ok(resp_json["public_token"].as_str().expect("failed to get public token").to_string())
        })
    }

    fn exchange_public_token(&self, public_token: &str) -> impl Future<Item=Value, Error=hyper::Error> {
        let url = "https://sandbox.plaid.com/item/public_token/exchange";
        let client_id = env::var("PLAID_CLIENT_ID").unwrap();
        let secret = env::var("PLAID_SECRET").unwrap();
        let json = json!({
            "public_token": public_token,
            "client_id": client_id,
            "secret": secret});
        let json_str = serde_json::to_string_pretty(&json).expect("exchange pub token json err");
        println!("Getting access token, json: {}", json_str);
        self.post_json(&json_str, url)
    }
}

pub fn run_plaid() -> Result<(), Box<Error>> {
    rt::run(rt::lazy(|| {
        let ch = ClientHandle::new().unwrap();
        let ch1 = Arc::new(ch);
        let ch2 = Arc::clone(&ch1);
        let ch3 = Arc::clone(&ch1);
        ch1.get_session_id()
          .and_then(move |sess_id| {
            ch2.get_public_token(&sess_id)
        }).and_then(move |pub_token| {
            ch3.exchange_public_token(&pub_token)
        }).and_then(|resp_json| {
            let access_token = resp_json["access_token"].as_str().expect("failed to get access token");
            let item_id = resp_json["item_id"].as_str().expect("failed to get item id");
            println!("Access Token: {}, Item Id: {}", access_token, item_id);
            Ok(())
        }).map_err(|e| {
            panic!("Error: {}", e)
        })
    }));
    Ok(())
}