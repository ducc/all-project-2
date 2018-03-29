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

    let mqtt_subtopics = env::var("MQTT_SUBTOPICS")?;

    {
        // create sql tables if they doesnt exist

        let conn = open_connection()?;

        for subtopic in mqtt_subtopics.split(",") {
            let statement = format!(
                "CREATE TABLE IF NOT EXISTS {} (
                     unix_time TEXT PRIMARY KEY,
                     reading BLOB NOT NULL
                 )", subtopic.replace("/", "_")
            );

            conn.execute(&statement, &[])?;
        }
    }

    // start mqtt client

    let opts = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        .set_broker(&env::var("MQTT_BROKER")?);

    let mqtt_topic = env::var("MQTT_TOPIC")?;
    let mqtt_topic_1 = mqtt_topic.clone();
    
    let callback = MqttCallback::new().on_message(move |msg| {
        let mqtt_topic_2 = mqtt_topic_1.clone();
        on_message(mqtt_topic_2, msg).expect("error handling mqtt message");
    });
    let mut request = MqttClient::start(opts, Some(callback))?;

    let mqtt_subtopics = mqtt_subtopics.split(",");
    mqtt_subtopics.clone().into_iter()
        .map(|subtopic| (String::from(mqtt_topic.as_ref()) + subtopic, QoS::Level0))
        .for_each(|(subtopic, qos)| {
            if let Err(e) = request.subscribe(vec![(&subtopic, qos)]) {
                error!("Error subscribing to topic {}: {:?}", &subtopic, e);
            }
        });

    // start http server
    let http_addr = env::var("API_ADDRESS")?.parse().unwrap();
    let mut core = Core::new()?;
    let handle = core.handle();

    let server = Http::new().serve_addr_handle(&http_addr, &handle, || Ok(Server {
        allowed_topics: env::var("MQTT_SUBTOPICS").unwrap().split(",")
            .map(ToString::to_string)
            //.map(|s| s.replace("/", "_"))
            .collect(),
    }))?;
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

fn on_message(parent_topic: String, msg: Message) -> Result<(), Error> {
    let topic: String = msg.topic.into();
    let topic = &topic[parent_topic.len()..];

    trace!("MSG: {:?}", msg.payload.as_slice());
    debug!("{}: {:?}", topic, parse_noise_level(msg.payload.as_slice())
            .expect("couldnt read mqtt noise_level"));

    let conn = open_connection()?;
    let statement = format!("INSERT INTO {} (unix_time, reading)
                     VALUES (?1, ?2)", &topic.replace("/", "_"));

    conn.execute(&statement, &[&Utc::now(), &*msg.payload])?;

    Ok(())
}

struct Server {
    allowed_topics: Vec<String>,
}

impl Service for Server {
    type Request = Request;
    type Response = Response;
    type Error = HyperError;
    type Future = FutureResult<Response, Self::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let response = Response::new()
            .with_header(AccessControlAllowOrigin::Any);

        if req.method() != &Get {
            return future::ok(response.with_status(StatusCode::MethodNotAllowed));
        }

        let target_topic = &req.path()[1..];
        println!("target topic: {}", &target_topic);
        if !self.allowed_topics.contains(&target_topic.to_string()) {
            return future::ok(response.with_status(StatusCode::NotFound));
        }

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

        let readings = match get_readings(&target_topic.replace("/", "_"), &query_from, &query_to) {
            Ok(readings) => readings,
            Err(e) => {
                error!("error querying readings for topic {}: {:?}", target_topic, e);
                return future::ok(response.with_status(StatusCode::BadRequest));
            }
        };

        let response_body = serde_json::to_string(&readings)
            .expect("could not serialize readings");

        future::ok(response
            .with_header(ContentType::json())
            .with_header(ContentLength(response_body.len() as u64))
            .with_body(response_body))       
    }

    /*fn call(&self, req: Request) -> Self::Future {
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
    }*/
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

fn get_readings(topic: &str, from: &DateTime<Utc>, to: &DateTime<Utc>) -> Result<Vec<(i64, f32)>, Error> {
    let conn = open_connection()?;

    let statement = format!("SELECT unix_time, reading FROM {}
                             WHERE datetime(unix_time) BETWEEN datetime(?1) AND datetime(?2)", topic);
    let mut stmt = conn.prepare(&statement)?;

    let result = stmt
        .query_map(&[from, to], |row| {
            debug!("got row");
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