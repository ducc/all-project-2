#![feature(try_trait)]

#[macro_use]
extern crate log;

extern crate rumqtt;
extern crate env_logger;
extern crate kankyo;
extern crate byteorder;
extern crate rusqlite;
extern crate time;
extern crate futures;
extern crate tokio_core;
extern crate hyper;
extern crate url;
extern crate chrono;

mod error;

use error::Error;
use rumqtt::{
    MqttOptions, 
    MqttClient, 
    MqttCallback, 
    QoS,
    Message
};
use byteorder::{ReadBytesExt, BigEndian};
use std::env;
use rusqlite::Connection;
use time::Duration;
use std::io::Error as IoError;
use tokio_core::reactor::Core;
use hyper::server::{Http, Service, Request, Response};
use hyper::{Get, StatusCode, Error as HyperError};
use hyper::header::ContentLength;
use futures::{Future, Stream};
use futures::future::{self, FutureResult};
use url::form_urlencoded;
use chrono::{DateTime, Utc};

fn main() {
    try_main().expect("oh no");
}

fn try_main() -> Result<(), Error> {
    env_logger::init();
    kankyo::load()?;

    // create sql table if it doesnt exist

    {
        let conn = open_connection()?;
        conn.execute("CREATE TABLE IF NOT EXISTS noise_levels (
                        unix_time TEXT PRIMARY KEY,
                        noise_level BLOB NOT NULL
                      )", &[])?;
        //conn.execute("DROP TABLE noise_levels", &[])?;

        // read existing values as a test
        //let mut stmt = conn.prepare("SELECT unix_time, noise_level FROM noise_levels")?;
        let mut stmt = conn.prepare("SELECT unix_time, noise_level FROM noise_levels
                                     WHERE datetime(unix_time) BETWEEN datetime(?1) AND datetime(?2)")?;

        stmt
            .query_map(&[&(Utc::now() - Duration::seconds(120)), &Utc::now()], |row| {
                let unix_time: DateTime<Utc> = row.get(0);
                let noise_level: Vec<u8> = row.get(1);
                (unix_time, noise_level)
            })?
            .filter_map(Result::ok)
            .for_each(|(unix_time, noise_level)| {
                let noise_level = parse_noise_level(noise_level.as_slice())
                    .expect("couldnt read db noise_level");

                debug!("unix_time: {:?} noise_level: {:?}", unix_time, noise_level);
            });
    }

    // start mqtt client

    let opts = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        .set_broker(&env::var("MQTT_BROKER")?);

    let callback = MqttCallback::new().on_message(|msg| {
        on_message(msg).expect("error handling mqtt message");
    });

    let mut request = MqttClient::start(opts, Some(callback))?;
    request.subscribe(vec![(&env::var("MQTT_TOPIC")?, QoS::Level0)])?;

    // start http server
    let http_addr = env::var("HTTP_ADDRESS")?.parse().unwrap();

    let mut core = Core::new()?;
    let handle = core.handle();

    let server = Http::new().serve_addr_handle(&http_addr, &handle, || Ok(Server {}))?;
    info!("listening on http://{}", server.incoming_ref().local_addr());

    let handle1 = handle.clone();
    handle.spawn(server.for_each(move |conn| {
        handle1
            .spawn(conn.map(|_| ())
            .map_err(|err| error!("server error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));

    let _ = core.run(future::empty::<(), ()>());

    Ok(())
}

fn open_connection() -> Result<Connection, Error> {
    Ok(Connection::open(&env::var("SQLITE_DATABASE")?)?)
}

fn parse_noise_level(mut payload: &[u8]) -> Result<f32, IoError> {
    payload.read_f32::<BigEndian>()
}

fn on_message(msg: Message) -> Result<(), Error> {
    debug!("noise level: {:?}", parse_noise_level(msg.payload.as_slice())
            .expect("couldnt read mqtt noise_level"));

    let conn = open_connection()?;

    conn.execute("INSERT INTO noise_levels (unix_time, noise_level)
                  VALUES (?1, ?2)",
                &[&Utc::now(), &*msg.payload])?;

    Ok(())
}

struct Server;

impl Service for Server {
    type Request = Request;
    type Response = Response;
    type Error = HyperError;
    type Future = FutureResult<Response, Self::Error>;

    fn call(&self, req: Request) -> Self::Future {
        future::ok(match (req.method(), req.path()) {
            (&Get, "/noise_levels") => {
                let query = match req.uri().query() {
                    Some(query) => query,
                    None => {
                        return future::ok(Response::new().with_status(StatusCode::BadRequest));
                    }
                };

                let (query_from, query_to) = match parse_query(query) {
                    Ok(pairs) => pairs,
                    Err(e) => {
                        error!("error parsing query {:?}", e);
                        return future::ok(Response::new().with_status(StatusCode::BadRequest));
                    }
                };

                let response = format!("Hello world!\nfrom: {}\nto: {}", 
                                        query_from, query_to);       

                Response::new()
                    .with_header(ContentLength(response.len() as u64))
                    .with_body(response)
            },
            _ => {
                Response::new()
                    .with_status(StatusCode::NotFound)
            }
        })       
    }
}

fn parse_query(query: &str) -> Result<(i64, i64), Error> {
    let pairs = form_urlencoded::parse(query.as_bytes());

    let mut from = None;
    let mut to = None;

    for (key, value) in pairs {
        match &*key {
            "from" => from = Some(value.parse::<i64>()?),
            "to" => to = Some(value.parse::<i64>()?),
            _ => {},
        }
    }
    
    Ok((from?, to.or(Some(0))?))
}

// todo 

// GET /noise_levels?from=0&to=0
// body format: [[unix_time, noise_level],...]
// data ascending in unix time
// no to value = now
//
// example
// stored unix_time: 2018-02-26 20:27:46:658244400 UTC
// Timespec { sec: 1519676866, nsec: 658244400 }
// format %Y-%m-%d %H:%M:%S:%f %Z

// potentially open websocket connection for streaming live data
// or just poll /noise_levels endpoint