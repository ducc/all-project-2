use std::fmt;
use rumqtt::Error as RumqttError;
use std::io::Error as IOError;
use std::env::VarError;
use rusqlite::Error as RusqliteError;

pub enum Error {
    Io(IOError),
    Rumqtt(RumqttError),
    Var(VarError),
    Rusqlite(RusqliteError)
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        use std::error::Error;

        write!(f, "{}", match *self {
            Io(ref err) => err.description(),
            Rumqtt(ref err) => err.description(),
            Var(ref err) => err.description(),
            Rusqlite(ref err) => err.description(),
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

impl From<VarError> for Error {
    fn from(err: VarError) -> Self {
        Error::Var(err)
    }
}

impl From<RusqliteError> for Error {
    fn from(err: RusqliteError) -> Self {
        Error::Rusqlite(err)
    }
}
