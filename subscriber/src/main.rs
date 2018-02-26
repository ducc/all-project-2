#[macro_use]
extern crate log;

extern crate rumqtt;
extern crate env_logger;
extern crate kankyo;
extern crate byteorder;
extern crate rusqlite;
extern crate time;

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
use time::Timespec;
use std::io::Error as IoError;

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

        // read existing values as a test
        let mut stmt = conn.prepare("SELECT unix_time, noise_level FROM noise_levels")?;
        
        stmt
            .query_map(&[], |row| {
                let unix_time: Timespec = row.get(0);
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

    loop {
        // inf loop to keep thread alive
    }
}

fn open_connection() -> Result<Connection, Error> {
    Ok(Connection::open(&env::var("SQLITE_DATABASE")?)?)
}

fn parse_noise_level(mut payload: &[u8]) -> Result<f32, IoError> {
    payload.read_f32::<BigEndian>()
}

fn on_message(msg: Message) -> Result<(), Error> {
    let noise_level = parse_noise_level(msg.payload.as_slice())
        .expect("couldnt read mqtt noise_level");

    info!("noise level: {:?}", noise_level);

    let conn = open_connection()?;

    conn.execute("INSERT INTO noise_levels (unix_time, noise_level)
                  VALUES (?1, ?2)",
                &[&time::get_time(), &*msg.payload])?;

    debug!("inserted into noise_levels");
    Ok(())
}