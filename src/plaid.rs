extern crate hyper;
extern crate rand;
use rand::{Rng};
use hyper::{Client, Method, Body, Request, StatusCode};
use hyper::client::{HttpConnector};
use hyper::header::{HeaderValue, HeaderMap};
use hyper::rt::{Future, Stream};
use hyper_tls::HttpsConnector;
use std::{env};
use std::error::Error;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value, Map};
use std::str::from_utf8;

const LINK_VERSION: &'static str = "2.0.264";
//pub const API_VERSION: &'static str = "2019-05-29";

const CREDENTIALS: Credentials =
    Credentials {
        username: "user_good",
        password: "pass_good" 
    };

#[derive(Debug, Serialize, Clone)]
pub struct Params {
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
    pub credentials: Option<Credentials>,
    #[serde(skip_serializing_if="Option::is_none")]
    public_token: Option<String>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AuthParams {
    pub access_token: Option<String>,
    #[serde(skip_serializing)]
    pub item_id: Option<String>,
    secret: Option<String>,
    client_id: Option<String>,
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
            display_language: None,
            flexible_input_responses: None,
            institution_id: None,
            options: None,
            credentials: None,
            public_token: None
        })
    }
}

impl AuthParams {
    pub fn new() -> Result<AuthParams, Box<Error>> {
        let client_id = env::var("PLAID_CLIENT_ID")?;
        let secret = env::var("PLAID_SECRET")?;
        Ok(AuthParams {
            access_token: None,
            item_id: None,
            secret: Some(secret),
            client_id: Some(client_id),
        })
    }
    fn add_json(&self, json_v: &Value) -> String {
        let json_map = json_v.as_object().unwrap();
        let mut json = serde_json::to_value(&self).unwrap();
        for (k,v) in json_map.iter() {
            json[k] = v.clone();
        }
        serde_json::to_string_pretty(&json).unwrap()
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Credentials {
    pub username: &'static str,
    password: &'static str,
}

const LINK_HEADERS: &'static [(&'static str, &'static str)] =
   &[ 
        ("Content-Type", "application/json"),
        ("User-Agent",
         "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:68.0) Gecko/20100101 Firefox/68.0"),
        ("Plaid-Link-Version", LINK_VERSION)
   ];

type HttpsClient = Client<HttpsConnector<HttpConnector>>;

#[derive(Debug)]
pub struct ClientHandle {
    pub headers: HeaderMap,
    pub params: Params,
    pub auth_params: AuthParams,
    client: HttpsClient,
}

impl ClientHandle {
    pub fn new() -> Result<ClientHandle, Box<Error>> {
        let mut headers = HeaderMap::new();
        LINK_HEADERS.iter().for_each(|h| {
            headers.insert(h.0, HeaderValue::from_static(h.1));
        });
        let https = HttpsConnector::new(4)?;
        let client = Client::builder().build::<_, hyper::Body>(https);
        Ok(ClientHandle {
            params: Params::new()?,
            auth_params: AuthParams::new()?,
            headers,
            client,
        })
    }
    
    fn post_json(&self, json: &str, uri: &str) -> impl Future<Item=Value, Error=String> {
        let uri: hyper::Uri = uri.parse().unwrap();
        let mut req = Request::new(Body::from(json.to_string()));
        *req.method_mut() = Method::POST;
        *req.uri_mut() = uri.clone();
        *req.headers_mut() = self.headers.clone();
        self.client.request(req).and_then(|res| {
                let status = res.status().clone();
                println!("Response: {}", status);
                res.into_body().concat2().and_then(move |body| Ok((status, body)))
            }).map_err(|e| e.to_string()).and_then(|(status, body)| {
                match status {
                    StatusCode::OK => {
                        let resp_json: Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
                        Ok(resp_json)
                    }
                    _ => Err(from_utf8(&body).map(|s| s.to_string())
                            .unwrap_or(format!("Bad status code: {}", status)))
                }
            })
    }
    
    pub fn get_session_id(mut self) -> impl Future<Item=Self, Error=String> {
        let json = serde_json::to_string_pretty(&self.params).unwrap();
        println!("Getting session id, json: {}", json);
        let url = "https://sandbox.plaid.com/link/client/get";
        self.post_json(&json, url).and_then(|resp_json| {
            self.params.link_session_id = Some(resp_json["link_session_id"].as_str().ok_or("failed to get session id")?.to_string());
            Ok(self)
        })
    }

    
    pub fn get_public_token(mut self) -> impl Future<Item=Self, Error=String> {
        self.params.link_version = None;
        self.params.country_codes = None;
        self.params.display_language = Some("en");
        self.params.flexible_input_responses = Some(Value::Null);
        self.params.institution_id = Some("ins_1");
        self.params.options =  Some(Map::new());
        self.params.credentials = Some(CREDENTIALS);
        let json = serde_json::to_string_pretty(&self.params).unwrap();
        println!("Getting public token, json: {}", json);
        let url = "https://sandbox.plaid.com/link/item/create";
        self.post_json(&json, url).and_then(|resp_json| {
            self.params.public_token =  Some(resp_json["public_token"].as_str().ok_or("error parsing public token")?.to_string());
            Ok(self)
        })
    }

    
    pub fn exchange_public_token(self) -> impl Future<Item=(ClientHandle, Value), Error=String> {
        let url = "https://sandbox.plaid.com/item/public_token/exchange";
        let json = json!({
            "public_token": self.params.public_token.clone().unwrap(),
            "client_id": self.auth_params.client_id,
            "secret": self.auth_params.secret
        });
        let json_str = serde_json::to_string_pretty(&json).expect("pub token json err");
        println!("Getting access token, json: {}", json_str);
        self.post_json(&json_str, url).and_then(|json| Ok((self, json)))
    }

    
    fn api_call(&self, url: &str, json: Value) -> impl Future<Item=Value, Error=String> {
        let json_str = self.auth_params.add_json(&json); 
        self.post_json(&json_str, url)
    }
    
    pub fn get_transactions(&self) -> impl Future<Item=Value, Error=String> {
        let url = "https://sandbox.plaid.com/transactions/get";
        let json = json!({
            "start_date": "2019-07-01",
            "end_date": "2019-08-10",
            "options": Map::new()
        });
        self.api_call(url, json)
    }

    pub fn get_balance(&self) -> impl Future<Item=Value, Error=String> {
        let url = "https://sandbox.plaid.com/accounts/balance/get";
        self.api_call(url, Map::new().into())
    }
}

pub fn get_access_token() -> impl Future<Item=(ClientHandle, Value), Error=String> {
    let ch = ClientHandle::new().unwrap();
    ch.get_session_id()
        .and_then(|ch| {
        ch.get_public_token()
    }).and_then(|ch| {
        ch.exchange_public_token()
    }).and_then(|(mut ch, json)| {
        ch.auth_params.access_token = Some(json["access_token"].as_str().ok_or("error parsing access token")?.to_string());
        ch.auth_params.item_id = Some(json["item_id"].as_str().ok_or("error parsing item id")?.to_string());
        Ok((ch, json))
    })
}

#[derive(Deserialize, Debug, Clone)]
pub struct Balance {
    pub available: Option<f32>,
    pub current: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Account {
    pub account_id: String,
    pub name: String,
    pub balances: Balance
}

#[derive(Deserialize, Debug, Clone)]
pub struct Accounts {
    pub accounts: Vec<Account>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Transaction {
    pub transaction_id: String,
    pub account_id: String,
    pub transaction_type: String,
    pub name: String,
    pub amount: f32,
    pub date: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Transactions {
    pub transactions: Vec<Transaction>
}