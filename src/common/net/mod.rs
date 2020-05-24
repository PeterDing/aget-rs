pub mod net;

use std::{boxed::Box, pin::Pin, time::Duration};

pub use http::{self, header, HeaderMap, HeaderValue, Method, Request, Response, Uri};

pub use url::Url;

pub use awc::{
    error as net_error, Client as HttpClient, ClientBuilder, ClientRequest, ClientResponse,
    Connector,
};

use bytes::Bytes;
use futures::Stream;

use actix_http::{encoding::Decoder, error::PayloadError, Payload};

pub type RClientResponse =
    ClientResponse<Decoder<Payload<Pin<Box<dyn Stream<Item = Result<Bytes, PayloadError>>>>>>>;

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
