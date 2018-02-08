extern crate rumqtt;

use rumqtt::{
    MqttOptions, 
    MqttClient, 
    MqttCallback, 
    QoS, 
    Error as RumqttError
};
use std::io::Error as IOError;
use std::fmt;

const TOPIC: &'static str = "testing12345/c";

fn main() {
    try_main().expect("oh no");

    loop {
        // inf loop to keep thread alive
    }
}

fn try_main() -> Result<(), Error> {
    println!("Hello, world!");

    let opts = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        .set_broker("iot.eclipse.org:1883");

    let callback = MqttCallback::new().on_message(move |message| {
        println!("message: {:?}", message);
    });

    let mut request = MqttClient::start(opts, Some(callback))?;
    request.subscribe(vec![(TOPIC, QoS::Level0)])?;

    Ok(())
}

enum Error {
    Io(IOError),
    Rumqtt(RumqttError),
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        use std::error::Error;

        write!(f, "{}", match *self {
            Io(ref err) => err.description(),
            Rumqtt(ref err) => err.description(),
        })
    }
}

impl From<IOError> for Error {
    fn from(err: IOError) -> Self {
        Error::Io(err)
    }
}

impl From<RumqttError> for Error {
    fn from(err: RumqttError) -> Self {
        Error::Rumqtt(err)
    }
}
