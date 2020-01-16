use std::{fmt, io::Error as IoError, num, result};

use failure::{self, Backtrace, Fail};
use futures::channel::mpsc::SendError;

use awc::{
    self,
    error::SendRequestError,
    http,
    http::{
        header::{InvalidHeaderName, InvalidHeaderValue, ToStrError},
        uri::InvalidUri,
    },
};

pub type Result<T, E = Error> = result::Result<T, E>;

pub struct Error {
    cause: Box<dyn AgetFail>,
    backtrace: Option<Backtrace>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.cause, f)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(bt) = self.cause.backtrace() {
            write!(f, "{:?}\n\n{:?}", &self.cause, bt)
        } else {
            write!(
                f,
                "{:?}\n\n{:?}",
                &self.cause,
                self.backtrace.as_ref().unwrap()
            )
        }
    }
}

pub trait AgetFail: Fail {}

impl<T: AgetFail> From<T> for Error {
    fn from(err: T) -> Error {
        let backtrace = if err.backtrace().is_none() {
            Some(Backtrace::new())
        } else {
            None
        };
        Error {
            cause: Box::new(err),
            backtrace,
        }
    }
}

#[derive(Fail, Debug)]
pub enum ArgError {
    #[fail(display = "Output path is invalid: {}", _0)]
    InvalidPath(String),
    #[fail(display = "the uri is invalid: {}", _0)]
    InvaildUri(String),
    #[fail(display = "No filename.")]
    NoFilename,
    #[fail(display = "Directory is not found")]
    NotFoundDirectory,
    #[fail(display = "The file already exists.")]
    FileExists,
    #[fail(display = "The path is a directory.")]
    PathIsDirectory,
    #[fail(display = "Can't parse string as number: {}", _0)]
    IsNotNumber(String),
    #[fail(display = "Io Error: {}", _0)]
    Io(#[cause] IoError),
}

impl AgetFail for ArgError {}

impl From<http::uri::InvalidUri> for ArgError {
    fn from(err: http::uri::InvalidUri) -> ArgError {
        ArgError::InvaildUri(format!("{}", err))
    }
}

impl From<num::ParseIntError> for ArgError {
    fn from(err: num::ParseIntError) -> ArgError {
        ArgError::IsNotNumber(format!("{}", err))
    }
}

impl From<IoError> for ArgError {
    fn from(err: IoError) -> ArgError {
        ArgError::Io(err)
    }
}

#[derive(Fail, Debug)]
pub enum AgetError {
    #[fail(display = "Method is unsupported")]
    UnsupportedMethod,
    #[fail(display = "header is invalid: {}", _0)]
    HeaderParseError(String),
    #[fail(display = "BUG: {}", _0)]
    Bug(String),
    #[fail(display = "Path is invalid: {}", _0)]
    InvalidPath(String),
    #[fail(display = "Io Error: {}", _0)]
    Io(#[cause] IoError),
    #[fail(display = "No filename.")]
    NoFilename,
    #[fail(display = "The file already exists.")]
    FileExists,
    #[fail(
        display = "The two content lengths are not equal between the response and the aget file."
    )]
    ContentLengthIsNotConsistent,
}

impl AgetFail for AgetError {}

impl From<IoError> for AgetError {
    fn from(err: IoError) -> AgetError {
        AgetError::Io(err)
    }
}

#[derive(Fail, Debug)]
pub enum NetError {
    #[fail(display = "an internal error: {}", _0)]
    ActixError(String),
    #[fail(display = "content does not has length")]
    NoContentLength,
    #[fail(display = "uri is invalid: {}", _0)]
    InvaildUri(String),
    #[fail(display = "header is invalid: {}", _0)]
    InvaildHeader(String),
    #[fail(display = "response status code is: {}", _0)]
    Unsuccess(u16),
    #[fail(display = "Redirect to: {}", _0)]
    Redirect(String),
}

impl AgetFail for NetError {}

impl From<SendRequestError> for NetError {
    fn from(err: SendRequestError) -> NetError {
        NetError::ActixError(format!("{}", err))
    }
}

impl From<ToStrError> for NetError {
    fn from(err: ToStrError) -> NetError {
        NetError::ActixError(format!("{}", err))
    }
}

impl From<InvalidUri> for NetError {
    fn from(err: InvalidUri) -> NetError {
        NetError::InvaildUri(format!("{}", err))
    }
}

impl From<InvalidHeaderName> for NetError {
    fn from(err: InvalidHeaderName) -> NetError {
        NetError::InvaildHeader(format!("{}", err))
    }
}

impl From<InvalidHeaderValue> for NetError {
    fn from(err: InvalidHeaderValue) -> NetError {
        NetError::InvaildHeader(format!("{}", err))
    }
}

impl From<SendError> for NetError {
    fn from(err: SendError) -> NetError {
        NetError::ActixError(format!("{}", err))
    }
}
