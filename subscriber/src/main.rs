extern crate rumqtt;
#[macro_use]
extern crate log;
extern crate env_logger;

mod error;

use error::Error;
use rumqtt::{
    MqttOptions, 
    MqttClient, 
    MqttCallback, 
    QoS,
    Message
};

const TOPIC: &'static str = "testing12345/c/noise/decibels/#";

fn main() {
    try_main().expect("oh no");
}

fn try_main() -> Result<(), Error> {
    env_logger::init();

    info!("Hello, world!");

    let opts = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        .set_broker("iot.eclipse.org:1883");

    let callback = MqttCallback::new().on_message(on_message);

    let mut request = MqttClient::start(opts, Some(callback))?;
    request.subscribe(vec![(TOPIC, QoS::Level0)])?;

    loop {
        // inf loop to keep thread alive
    }
}

fn on_message(msg: Message) {
    info!("message: {:?}", msg);
}
