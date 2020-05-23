pub use isahc::{
    self,
    config::Configurable,
    http,
    http::{header, HeaderMap, HeaderValue, Method, Request, Response, Uri},
    Body, HttpClient, RequestExt, ResponseExt,
};

pub use url::Url;

#[derive(Debug)]
pub enum ContentLengthValue {
    RangeLength(u64),
    DirectLength(u64),
    NoLength,
}
