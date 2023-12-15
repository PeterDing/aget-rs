pub mod net;

use std::time::Duration;

pub use http::Uri;
pub use reqwest::{
    header::{HeaderMap, HeaderName},
    Client as HttpClient, Method, Request, Response,
};
pub use url::Url;

#[derive(Debug)]
pub enum ContentLengthValue {
    RangeLength(u64),
    DirectLength(u64),
    NoLength,
}

pub struct ConnectorConfig {
    pub timeout: Duration,
    pub dns_timeout: Duration,
    pub keep_alive: Duration,
    pub lifetime: Duration,
    pub disable_redirects: bool,
}
