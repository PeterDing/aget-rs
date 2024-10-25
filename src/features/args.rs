use std::{path::PathBuf, time::Duration};

use crate::common::{
    net::{Method, Url},
    tasks::TaskType,
};

/// This a arg which gives parameters for apps
pub trait Args {
    /// Path of output
    fn output(&self) -> PathBuf;

    /// Request method for http
    fn method(&self) -> Method;

    /// The uri of a task
    fn url(&self) -> Url;

    /// The data for http post request
    fn data(&self) -> Option<&str>;

    /// Request headers
    fn headers(&self) -> Vec<(&str, &str)>;

    /// Proxy: http, https, socks4, socks5
    fn proxy(&self) -> Option<&str>;

    /// The maximum time the request is allowed to take.
    fn timeout(&self) -> Duration;

    /// Connection timeout
    ///
    /// i.e. max time to connect to remote host including dns name resolution.
    /// Set to 1 second by default.
    fn dns_timeout(&self) -> Duration;

    /// Set keep-alive period for opened connection.
    ///
    /// Keep-alive period is the period between connection usage. If
    /// the delay between repeated usages of the same connection
    /// exceeds this period, the connection is closed.
    /// Default keep-alive period is 15 seconds.
    fn keep_alive(&self) -> Duration;

    /// Set max lifetime period for connection.
    ///
    /// Connection lifetime is max lifetime of any opened connection
    /// until it is closed regardless of keep-alive period.
    /// Default lifetime period is 75 seconds.
    fn lifetime(&self) -> Duration;

    /// Always return `true`
    fn disable_redirects(&self) -> bool;

    /// Skip to verify the server's TLS certificate
    fn skip_verify_tls_cert(&self) -> bool;

    /// The number of concurrency
    fn concurrency(&self) -> u64;

    /// The chunk size of each concurrency for http task
    fn chunk_size(&self) -> u64;

    /// The number of retry of a task
    fn retries(&self) -> u64;

    /// The internal of each retry
    fn retry_wait(&self) -> u64;

    /// Task type
    fn task_type(&self) -> TaskType;

    /// A regex to only download files matching it in the torrent
    fn bt_file_regex(&self) -> Option<String>;

    /// Seed the torrent
    fn seed(&self) -> bool;

    /// Trackers for the torrent
    fn bt_trackers(&self) -> Option<Vec<String>>;

    /// Peer connect timeout
    fn bt_peer_connect_timeout(&self) -> Option<u64>;

    /// Peer read/write timeout
    fn bt_peer_read_write_timeout(&self) -> Option<u64>;

    /// Peer keep alive interval
    fn bt_peer_keep_alive_interval(&self) -> Option<u64>;

    /// To debug mode, if it returns true
    fn debug(&self) -> bool;

    /// To quiet mode, if it return true
    fn quiet(&self) -> bool;
}
