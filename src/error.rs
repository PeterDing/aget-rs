use std::io::Error as IoError;
use std::num;
use std::{fmt, result};

use failure::{self, Backtrace, Compat, Fail};

use actix_web;
use actix_web::http;

pub type Result<T, E = Error> = result::Result<T, E>;

pub struct Error {
    cause: Box<AgetFail>,
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

impl<T> AgetFail for Compat<T> where T: fmt::Display + fmt::Debug + Sync + Send + 'static {}

impl From<failure::Error> for Error {
    fn from(err: failure::Error) -> Error {
        err.compat().into()
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
    #[fail(display = "an internal error.")]
    ActixError,
    #[fail(display = "content does not has length")]
    NoContentLength,
    #[fail(display = "uri is invalid: {}", _0)]
    InvaildUri(String),
}

impl AgetFail for NetError {}

impl From<actix_web::Error> for NetError {
    fn from(err: actix_web::Error) -> NetError {
        NetError::ActixError
    }
}

// impl<T> From<T> for NetError
// where
// T: actix_web::error::ResponseError,
// {
// fn from(err: T) -> NetError {
// let cause = err.as_fail();
// NetError::ActixError
// }
// }

impl From<actix_web::client::SendRequestError> for NetError {
    fn from(err: actix_web::client::SendRequestError) -> NetError {
        NetError::ActixError
    }
}

impl From<actix_web::error::PayloadError> for NetError {
    fn from(err: actix_web::error::PayloadError) -> NetError {
        NetError::ActixError
    }
}

impl From<http::uri::InvalidUri> for NetError {
    fn from(err: http::uri::InvalidUri) -> NetError {
        NetError::InvaildUri(format!("{}", err))
    }
}
