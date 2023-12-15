pub mod net;

pub use http::Uri;
pub use reqwest::{
    header::{HeaderMap, HeaderName},
    Client as HttpClient, Method, Proxy, Request, Response,
};
pub use url::Url;

#[derive(Debug)]
pub enum ContentLengthValue {
    RangeLength(u64),
    DirectLength(u64),
    NoLength,
}
