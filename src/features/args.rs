use std::path::PathBuf;

use crate::common::{
    bytes::bytes_type::BytesMut,
    net::net_type::{Method, Uri},
    tasks::TaskType,
};

/// This a arg which gives parameters for apps
pub trait Args {
    /// Path of output
    fn output(&self) -> PathBuf;

    /// Request method for http
    fn method(&self) -> Method;

    /// The uri of a task
    fn uri(&self) -> Uri;

    /// The data for http post request
    fn data(&self) -> Option<BytesMut>;

    /// Request headers
    fn headers(&self) -> Vec<(String, String)>;

    /// Proxy: http, https, socks4, socks5
    fn proxy(&self) -> Option<String>;

    /// The maximum time the request is allowed to take.
    fn timeout(&self) -> u64;

    /// The number of concurrency
    fn concurrency(&self) -> u64;

    /// The chunk length of each concurrency for http task
    fn chunk_length(&self) -> u64;

    /// The number of retry of a task
    fn retries(&self) -> u64;

    /// The internal of each retry
    fn retry_wait(&self) -> u64;

    /// Task type
    fn task_type(&self) -> TaskType;

    /// To debug mode, if it returns true
    fn debug(&self) -> bool;

    /// To quiet mode, if it return true
    fn quiet(&self) -> bool;
}
