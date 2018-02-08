extern crate rumqtt;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate kankyo;
extern crate byteorder;

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

fn main() {
    try_main().expect("oh no");
}

fn try_main() -> Result<(), Error> {
    env_logger::init();
    kankyo::load()?;

    let opts = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        .set_broker(&env::var("MQTT_BROKER")?);

    let callback = MqttCallback::new().on_message(on_message);

    let mut request = MqttClient::start(opts, Some(callback))?;
    request.subscribe(vec![(&env::var("MQTT_TOPIC")?, QoS::Level0)])?;

    loop {
        // inf loop to keep thread alive
    }
}

fn on_message(msg: Message) {
    let bytes = msg.payload.as_slice().read_f32::<BigEndian>().expect("couldnt read f64");
    info!("message: {:?}", bytes);
}
