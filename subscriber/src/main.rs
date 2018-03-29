#![feature(try_trait)]

// here we import all the libraries that the project is using
// these are defined in the Cargo.toml file
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

// importing our error.rs file 
mod error;

// importing types from library modules and local modules
use error::Error;
use rumqtt::{
    MqttOptions, 
    MqttClient, 
    MqttCallback, 
    QoS,
    Message
};
use std::env;
use rusqlite::Connection;
use tokio_core::reactor::Core;
use hyper::server::{Http, Service, Request, Response};
use hyper::{Get, StatusCode, Error as HyperError};
use hyper::header::{ContentLength, ContentType, AccessControlAllowOrigin};
use futures::{Future, Stream};
use futures::future::{self, FutureResult};
use url::form_urlencoded;
use chrono::{DateTime, Utc, NaiveDateTime};

// the program main entry point
// a second try_main function is used returning a result which is a common rust design pattern
fn main() {
    try_main().expect("oh no");
}

// this function returns Result<(), Error> which means if the program runs successfully 
// nothing is returned, but otherwise the Error type is returned
fn try_main() -> Result<(), Error> {
    // loading environment variables from a .env file using the kankyo library
    kankyo::load()?;

    // initializing a logging implementation that uses environment variables for log scopes
    env_logger::init();

    // the mqtt topics we will be subscribing to
    let mqtt_subtopics = env::var("MQTT_SUBTOPICS")?;

    {
        // create sql tables if they doesnt exist
        // open an sql connection to our sqlite3 database
        let conn = open_connection()?;

        // in the .env file topics are stored in the format of topic/a,topic/b,topic/c
        // so here we split by the comma delimiter
        for subtopic in mqtt_subtopics.split(",") {
            // dynamically creating the sql statement
            // this could be a potential security issue and in a production system the tables
            // should be predefined
            let statement = format!(
                "CREATE TABLE IF NOT EXISTS {} (
                     unix_time TEXT PRIMARY KEY,
                     reading BLOB NOT NULL
                 )", subtopic.replace("/", "_")
            );

            // executing the sql statements via our connection
            conn.execute(&statement, &[])?;
        }
    }

    // start mqtt client
    let opts = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        // the host mqtt server that messages are sent on
        .set_broker(&env::var("MQTT_BROKER")?);

    // parent topic for all sub topics to be under
    let mqtt_topic = env::var("MQTT_TOPIC")?;
    // rust's ownership and thread safety system requires that the topic is cloned 
    // for it to be used in the callback closure below
    let mqtt_topic_1 = mqtt_topic.clone();
    
    // creating a callback for the mqtt library
    // when a message is received this function will be invoked
    let callback = MqttCallback::new().on_message(move |msg| {
        let mqtt_topic_2 = mqtt_topic_1.clone();
        on_message(mqtt_topic_2, msg).expect("error handling mqtt message");
    });
    // connects to the mqtt broker
    let mut request = MqttClient::start(opts, Some(callback))?;

    // subscribing to each mqtt topic to get readings from
    let mqtt_subtopics = mqtt_subtopics.split(",");
    mqtt_subtopics.clone().into_iter()
        // functional programming style mapping converting from an input value to an output value
        // "noise/decibels" -> (&"noise/decibels", QoS::Level0)
        // rust has 2 string types which often have to be converted to and from
        // QoS::Level0 means that we want the mqtt protocol to receive messages with a 
        // quality of service level of 0
        .map(|subtopic| (String::from(mqtt_topic.as_ref()) + subtopic, QoS::Level0))
        // iterate each tuple
        .for_each(|(subtopic, qos)| {
            // subscribe to the topic and if there has been an error output it to our error logger
            if let Err(e) = request.subscribe(vec![(&subtopic, qos)]) {
                error!("Error subscribing to topic {}: {:?}", &subtopic, e);
            }
        });

    // get the http server address environment variable and parse it as a socket address
    let http_addr = env::var("API_ADDRESS")?.parse().unwrap();

    // create an async event loop for the http server to run on
    let mut core = Core::new()?;
    // get a handle to the event loop as a way of executing tasks on the loop, like a remote control
    let handle = core.handle();

    // initialize our server, specifying the configured mqtt subtopics to listen for in the 
    // Server structure initializer
    let server = Http::new().serve_addr_handle(&http_addr, &handle, || Ok(Server {
        allowed_topics: env::var("MQTT_SUBTOPICS").unwrap().split(",")
            // converting to &str (a reference to a String) to a String
            .map(ToString::to_string)
            // pushing all the values into a vector
            .collect(),
    }))?;

    // log that the server is starting with the address it is starting on
    info!("listening on http://{}", server.incoming_ref().local_addr());

    // copying a pointer to the event loop handle so it can be passed to the closure below
    let handle1 = handle.clone();
    // spawn a task on the event loop which iterates each incoming connection 
    // and spawns each connection as another task on the event loop so that it does not block
    // the processing of the server
    handle.spawn(server.for_each(move |conn| {
        handle1
            .spawn(conn.map(|_| ())
            .map_err(|err| error!("server error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));

    // running the event loop indefinitely with an empty task that does not resolve
    let _ = core.run(future::empty::<(), ()>());

    Ok(())
}

// opens a new connection to the sqlite3 database
fn open_connection() -> Result<Connection, Error> {
    Ok(Connection::open(&env::var("SQLITE_DATABASE")?)?)
}

// takes an byte array as input, produces a 32 bit float
fn parse_noise_level(payload: &[u8]) -> Result<f32, Error> {
    // the byte array is a utf8 string produced by the 'Sensor Node Free' app
    let payload_str = String::from_utf8(payload.to_vec())?;
    // if no data is read by the app it produces the string '-Infinity' so we return
    // 0 to compensate for that
    if payload_str == "-Infinity" {
        return Ok(0f32) // 0f32 is a short way of expressing that the number 0 is a 32 bit float
    }
    // try to parse the string as a 32 bit float...
    payload_str.parse::<f32>()
        // ...and convert parsing errors to our error type defined in the error module
        .map_err(From::from)
}

// when a message is received over mqtt this function is invoked as a callback
fn on_message(parent_topic: String, msg: Message) -> Result<(), Error> {
    // to get the topic this message was sent on it we must convert the library type
    // 'mqtt::topic_name::TopicName' to String by using the Into<String> implementation on
    // the type
    let topic: String = msg.topic.into();
    // substring the length of the parent topic from the topic name to get the subtopic name
    let topic = &topic[parent_topic.len()..];

    // send a log message at the trace level for debugging with the message topic & content
    trace!("{}: message {:?}", topic, msg.payload.as_slice());
    // call the pass noise level function on the received data and handle its result
    let value = match parse_noise_level(msg.payload.as_slice()) {
        Ok(value) => {
            // if the function runs successfully check if the value is 0
            // and if so return early from the function in order to not continue
            if value == 0f32 {
                return Ok(());
            }
            // otherwise move value to the outer scope
            value
        },
        // if an error occurs panic to exit the program early with an error message
        // as this should not happen
        Err(e) => panic!("error parsing noise level: {:?}", e),
    };
    // log a debug message with the parsed value
    debug!("{}: value   {:?}", topic, value);
    
    let conn = open_connection()?;
    // insert the received value into the topic's table
    // potential security risk as {} could be exploited with SQL injection
    // if the MQTT_SUBTOPICS environment variable is set to exploit it
    let statement = format!("INSERT INTO {} (unix_time, reading)
                     VALUES (?1, ?2)", &topic.replace("/", "_"));

    // execute the statement
    conn.execute(&statement, &[&Utc::now(), &*msg.payload])?;

    Ok(())
}

// a structure representing our server
struct Server {
    // the topics that are allowed to be queried over the rest api
    allowed_topics: Vec<String>,
}

// the hyper library requires the Service trait (like an interface in Java) to be
// implemented for the Server structure
// this works as the server handler when an http request is sent to the server
impl Service for Server {
    // defining the types as requested by the hyper library
    type Request = Request;
    type Response = Response;
    type Error = HyperError;
    type Future = FutureResult<Response, Self::Error>;

    // this function is called when a new connection is made to the server
    fn call(&self, req: Request) -> Self::Future {
        // set the access control response header so a web interface can call the api
        // via an XmlHttpRequest
        let response = Response::new()
            .with_header(AccessControlAllowOrigin::Any);

        // only the GET method is supported by this api so respond with the 'method not allowed'
        // status code and exit the function early
        if req.method() != &Get {
            return future::ok(response.with_status(StatusCode::MethodNotAllowed));
        }

        // get the path e.g. /noise/decibels and remove the prefix '/'
        let target_topic = &req.path()[1..];
        // check if the topic is an allowed topic as defined in the Server structure 
        // 'allowed_topics' field
        if !self.allowed_topics.contains(&target_topic.to_string()) {
            return future::ok(response.with_status(StatusCode::NotFound));
        }

        // unwrap the query which is an Option
        // this is the rust alternative to null with None being the equivelent
        // this pattern encourages checking of potential null values
        let query = match req.uri().query() {
            Some(query) => query,
            None => {
                return future::ok(response.with_status(StatusCode::BadRequest));
            }
        };

        // parse the inputted query parameters
        let (query_from, query_to) = match parse_query(query) {
            Ok(pairs) => pairs,
            Err(e) => {
                // if there is an issue parsing exit the function early
                println!("query: {}", query);
                error!("error parsing query: {:?}", e);
                return future::ok(response.with_status(StatusCode::BadRequest));
            }
        };

        // query the sqlite database for sensor readings of the specified topic
        // within the duration specified by the from and to query parameters
        let readings = match get_readings(&target_topic.replace("/", "_"), &query_from, &query_to) {
            Ok(readings) => readings,
            Err(e) => {
                error!("error querying readings for topic {}: {:?}", target_topic, e);
                return future::ok(response.with_status(StatusCode::BadRequest));
            }
        };

        // serialize the readings as json using the serde_json library
        let response_body = serde_json::to_string(&readings)
            .expect("could not serialize readings");

        // return the response body with appropriate content type and content length headers
        future::ok(response
            .with_header(ContentType::json())
            .with_header(ContentLength(response_body.len() as u64))
            .with_body(response_body))       
    }
}

// parse input query parameters as a duration represented returned value 1 and 2
fn parse_query(query: &str) -> Result<(DateTime<Utc>, DateTime<Utc>), Error> {
    // query parameters are formatted using the form url encoded format
    // so here the rust library for encoding/decoding is used to parse the string query input
    let pairs = form_urlencoded::parse(query.as_bytes());

    let mut from = None;
    let mut to = None;

    // iterating the query pairs assigning values to the above from & to temporary values
    for (key, value) in pairs {
        match &*key {
            // the strings are parsed as 64 bit integers
            "from" => from = Some(value.parse::<i64>()?),
            "to" => to = Some(value.parse::<i64>()?),
            _ => {},
        }
    }

    // parse 64 bit integer timestamps as rust DateTime<Utc> instances
    let from = parse_timestamp(from?)?;
    // if the to variable has no value default to using 0
    let to = parse_timestamp(to.or(Some(0)).unwrap())?;

    // return the from and to values in a tuple
    Ok((from, to))
}

// parses 64 bit integer timestamps as DateTime<Utc> instances
fn parse_timestamp(timestamp: i64) -> Result<DateTime<Utc>, Error> {
    Ok(DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp, 0), Utc))
}

// query the sqlite3 database for sensor readings within the duration defined by from and to for the given topic
fn get_readings(topic: &str, from: &DateTime<Utc>, to: &DateTime<Utc>) -> Result<Vec<(i64, f32)>, Error> {
    let conn = open_connection()?;

    let statement = format!("SELECT unix_time, reading FROM {}
                             WHERE datetime(unix_time) BETWEEN datetime(?1) AND datetime(?2)", topic);
    let mut stmt = conn.prepare(&statement)?;

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