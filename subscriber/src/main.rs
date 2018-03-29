#![feature(try_trait)]

#[macro_use]
extern crate log;
extern crate serde_json;

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
use std::io::Error as IoError;
use tokio_core::reactor::Core;
use hyper::server::{Http, Service, Request, Response};
use hyper::{Get, StatusCode, Error as HyperError};
use hyper::header::{ContentLength, ContentType, AccessControlAllowOrigin};
use futures::{Future, Stream};
use futures::future::{self, FutureResult};
use url::form_urlencoded;
use chrono::{DateTime, Utc, NaiveDateTime};

fn main() {
    try_main().expect("oh no");
}

fn try_main() -> Result<(), Error> {
    kankyo::load()?;
    env_logger::init();

    {
        // create sql table if it doesnt exist

        let conn = open_connection()?;
        conn.execute("CREATE TABLE IF NOT EXISTS noise_levels (
                        unix_time TEXT PRIMARY KEY,
                        noise_level BLOB NOT NULL
                      )", &[])?;
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

    let http_addr = env::var("API_ADDRESS")?.parse().unwrap();

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
        let response = Response::new()
            .with_header(AccessControlAllowOrigin::Any);

        future::ok(match (req.method(), req.path()) {
            (&Get, "/noise_levels") => {
                let query = match req.uri().query() {
                    Some(query) => query,
                    None => {
                        return future::ok(response.with_status(StatusCode::BadRequest));
                    }
                };

                let (query_from, query_to) = match parse_query(query) {
                    Ok(pairs) => pairs,
                    Err(e) => {
                        error!("error parsing query {:?}", e);
                        return future::ok(response.with_status(StatusCode::BadRequest));
                    }
                };

                let noise_levels = match get_noise_levels(&query_from, &query_to) {
                    Ok(noise_levels) => noise_levels,
                    Err(e) => {
                        error!("error querying noise levels {:?}", e);
                        return future::ok(response.with_status(StatusCode::BadRequest));
                    }
                };

                let response_body = serde_json::to_string(&noise_levels)
                    .expect("could not serialize noise levels");
                
                response
                    .with_header(ContentType::json())
                    .with_header(ContentLength(response_body.len() as u64))
                    .with_body(response_body)
            },
            _ => {
                response
                    .with_status(StatusCode::NotFound)
            }
        })       
    }
}

fn parse_query(query: &str) -> Result<(DateTime<Utc>, DateTime<Utc>), Error> {
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

    let from = parse_timestamp(from?)?;
    let to = parse_timestamp(to.or(Some(0)).unwrap())?;

    Ok((from, to))
}

fn parse_timestamp(timestamp: i64) -> Result<DateTime<Utc>, Error> {
    Ok(DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp, 0), Utc))
}

fn get_noise_levels(from: &DateTime<Utc>, to: &DateTime<Utc>) -> Result<Vec<(i64, f32)>, Error> {
    let conn = open_connection()?;

    let mut stmt = conn.prepare("SELECT unix_time, noise_level FROM noise_levels
                                 WHERE datetime(unix_time) BETWEEN datetime(?1) AND datetime(?2)")?;

    let result = stmt
        .query_map(&[from, to], |row| {
            let unix_time: DateTime<Utc> = row.get(0);
            let noise_level: Vec<u8> = row.get(1);
            (unix_time, noise_level)
        })?
        .filter_map(Result::ok)
        .map(|(unix_time, noise_level)| {
            (unix_time.timestamp(), parse_noise_level(noise_level.as_slice())
                .expect("couldnt read db noise_level"))
        })
        .collect::<Vec<_>>();

    Ok(result)
}

// todo 

// potentially open websocket connection for streaming live data
// or just poll /noise_levels endpoint