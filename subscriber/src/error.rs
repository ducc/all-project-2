use rumqtt::Error as RumqttError;
use std::io::Error as IOError;
use std::fmt;

pub enum Error {
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
