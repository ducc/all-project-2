use std::fmt;
use rumqtt::Error as RumqttError;
use std::io::Error as IOError;
use std::env::VarError;
use rusqlite::Error as RusqliteError;
use hyper::Error as HyperError;
use std::option::NoneError;
use std::num::ParseIntError;

pub enum Error {
    Io(IOError),
    Rumqtt(RumqttError),
    Var(VarError),
    Rusqlite(RusqliteError),
    Hyper(HyperError),
    None(NoneError),
    ParseInt(ParseIntError),
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
            Hyper(ref err) => err.description(),
            None(_) => "std::option::Option value not present",
            ParseInt(ref err) => err.description(),
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

impl From<HyperError> for Error {
    fn from(err: HyperError) -> Self {
        Error::Hyper(err)
    }
}

impl From<NoneError> for Error {
    fn from(err: NoneError) -> Self {
        Error::None(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Error::ParseInt(err)
    }
}