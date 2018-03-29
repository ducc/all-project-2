use std::fmt;
use rumqtt::Error as RumqttError;
use std::io::Error as IOError;
use std::env::VarError;
use rusqlite::Error as RusqliteError;
use hyper::Error as HyperError;
use std::option::NoneError;
use std::num::ParseIntError;
use std::num::ParseFloatError;
use std::string::FromUtf8Error;

pub enum Error {
    Io(IOError),
    Rumqtt(RumqttError),
    Var(VarError),
    Rusqlite(RusqliteError),
    Hyper(HyperError),
    None(NoneError),
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    FromUtf8(FromUtf8Error),
    InvalidValue,
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
            ParseFloat(ref err) => err.description(),
            FromUtf8(ref err) => err.description(),
            InvalidValue => "given data value is invalid"
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

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Self {
        Error::ParseFloat(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Self {
        Error::FromUtf8(err)
    }
}