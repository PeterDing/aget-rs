use std::{io::Error as IoError, num, result};

use thiserror::Error as ThisError;

use url::ParseError as UrlParseError;

use awc::error::{PayloadError, SendRequestError};

use openssl;

pub type Result<T, E = Error> = result::Result<T, E>;

#[derive(Debug, ThisError)]
pub enum Error {
    // For Arguments
    #[error("Output path is invalid: {0}")]
    InvalidPath(String),
    #[error("Uri is invalid: {0}")]
    InvaildUri(#[from] http::uri::InvalidUri),
    #[error("Header is invalid: {0}")]
    InvalidHeader(String),
    #[error("No filename.")]
    NoFilename,
    #[error("Directory is not found")]
    NotFoundDirectory,
    #[error("The file already exists.")]
    FileExists,
    #[error("The path is a directory.")]
    PathIsDirectory,
    #[error("Can't parse string as number: {0}")]
    IsNotNumber(#[from] num::ParseIntError),
    #[error("Io Error: {0}")]
    Io(#[from] IoError),
    #[error("{0} task is not supported")]
    UnsupportedTask(String),

    // For IO
    #[error("IO: Unexpected EOF")]
    UnexpectedEof,

    #[error("Procedure timeout")]
    Timeout,

    // For Network
    #[error("Network error: {0}")]
    NetError(String),
    #[error("Uncompleted Read")]
    UncompletedRead,
    #[error("{0} is unsupported")]
    UnsupportedMethod(String),
    #[error("header is invalid: {0}")]
    HeaderParseError(String),
    #[error("header is invalid: {0}")]
    UrlParseError(#[from] UrlParseError),
    #[error("BUG: {0}")]
    Bug(String),
    #[error("The two content lengths are not equal between the response and the aget file.")]
    ContentLengthIsNotConsistent,

    // For m3u8
    #[error("Fail to parse m3u8 file.")]
    M3U8ParseFail,
    #[error("The two m3u8 parts are not equal between the response and the aget file.")]
    PartsAreNotConsistent,

    #[error("An internal error: {0}")]
    InnerError(String),
    #[error("Content does not has length")]
    NoContentLength,
    #[error("header is invalid: {0}")]
    InvaildHeader(String),
    #[error("response status code is: {0}")]
    Unsuccess(u16),
    #[error("Redirect to: {0}")]
    Redirect(String),
    #[error("No Location for redirection: {0}")]
    NoLocation(String),
    #[error("Fail to decrypt aes128 data: {0}")]
    AES128DecryptFail(#[from] openssl::error::ErrorStack),
}

impl From<http::header::ToStrError> for Error {
    fn from(err: http::header::ToStrError) -> Error {
        Error::NetError(format!("{}", err))
    }
}

impl From<http::Error> for Error {
    fn from(err: http::Error) -> Error {
        Error::NetError(format!("{}", err))
    }
}

impl From<SendRequestError> for Error {
    fn from(err: SendRequestError) -> Error {
        Error::NetError(format!("{}", err))
    }
}

impl From<PayloadError> for Error {
    fn from(err: PayloadError) -> Error {
        Error::NetError(format!("{}", err))
    }
}
