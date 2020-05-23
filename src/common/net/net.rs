use std::time;

use crate::common::{
    bytes::bytes_type::Bytes,
    errors::{Error, Result},
    net::net_type::{
        header, Body, Configurable, ContentLengthValue, HttpClient, Method, Request, Response, Uri,
    },
    range::RangePair,
};

pub fn parse_header(raw: &str) -> Result<(&str, &str), Error> {
    if let Some(index) = raw.find(": ") {
        return Ok((&raw[..index], &raw[index + 2..]));
    }
    if let Some(index) = raw.find(":") {
        return Ok((&raw[..index], &raw[index + 1..]));
    }
    Err(Error::InvalidHeader(raw.to_string()))
}

pub fn parse_headers<'a, I: IntoIterator<Item = &'a str>>(
    raws: I,
) -> Result<Vec<(&'a str, &'a str)>, Error> {
    let mut headers = vec![];
    for raw in raws {
        let pair = parse_header(raw)?;
        headers.push(pair);
    }
    Ok(headers)
}

/// Builder a http client of curl
pub fn build_http_client(
    headers: &[(&str, &str)],
    timeout: u64,
    proxy: Option<&str>,
) -> Result<HttpClient> {
    let mut builder = HttpClient::builder().default_headers(headers).proxy({
        if proxy.is_some() {
            Some(proxy.unwrap().parse()?)
        } else {
            None
        }
    });
    // If timeout is zero, no timeout will be enforced.
    if timeout > 0 {
        builder = builder.timeout(time::Duration::from_secs(timeout));
    }
    let client = builder.cookies().build()?;
    Ok(client)
}

// TODO
struct RequestInfo {
    method: Method,
    uri: String,
    headers: Vec<(String, String)>,
    // Post data
    data: Option<Bytes>,
    timeout: u64,
    proxy: Option<String>,
}

impl RequestInfo {}

/// Check whether the response is success
/// Check if status is within 200-299.
pub fn is_success<T>(resp: &Response<T>) -> Result<(), Error> {
    let status = resp.status();
    if !status.is_success() {
        Err(Error::Unsuccess(status.as_u16()))
    } else {
        Ok(())
    }
}

/// Send a request with a range header, returning the final uri
pub async fn redirect(
    client: &HttpClient,
    method: Method,
    uri: Uri,
    data: Option<Bytes>,
) -> Result<Uri> {
    let mut uri = uri;
    loop {
        let data = data.clone().map(|d| Body::from_bytes(&d));
        let request = Request::builder()
            .method(method.clone())
            .uri(uri.clone())
            .header(header::RANGE, "bytes=0-1")
            .body(data)?;
        let resp = client.send_async(request).await?;
        if !resp.status().is_redirection() {
            break;
        }
        let headers = resp.headers();
        if let Some(location) = headers.get(header::LOCATION) {
            uri = location.to_str()?.parse()?;
        } else {
            break;
        }
    }
    Ok(uri)
}

/// Get the content length of the resource
pub async fn content_length(
    client: &HttpClient,
    method: Method,
    uri: Uri,
    data: Option<Bytes>,
) -> Result<ContentLengthValue> {
    let mut uri = uri;
    loop {
        let data = data.clone().map(|d| Body::from_bytes(&d));
        let request = Request::builder()
            .method(method.clone())
            .uri(uri.clone())
            .header(header::RANGE, "bytes=0-1")
            .body(data)?;
        let resp = client.send_async(request).await?;
        let headers = resp.headers();
        if resp.status().is_redirection() {
            if let Some(location) = headers.get(header::LOCATION) {
                uri = location.to_str()?.parse()?;
                continue;
            } else {
                return Err(Error::NoLocation(format!("{}", uri)));
            }
        } else {
            is_success(&resp)?;
            if let Some(h) = headers.get(header::CONTENT_RANGE) {
                if let Ok(s) = h.to_str() {
                    if let Some(index) = s.find("/") {
                        if let Ok(length) = &s[index + 1..].parse::<u64>() {
                            return Ok(ContentLengthValue::RangeLength(length.clone()));
                        }
                    }
                }
            }
            if let Some(h) = resp.headers().get(header::CONTENT_LENGTH) {
                if let Ok(s) = h.to_str() {
                    if let Ok(length) = s.parse::<u64>() {
                        return Ok(ContentLengthValue::DirectLength(length.clone()));
                    }
                }
            }
            break;
        }
    }
    Ok(ContentLengthValue::NoLength)
}

/// Send a request
pub async fn request(
    client: &HttpClient,
    method: Method,
    uri: Uri,
    data: Option<Bytes>,
    range: Option<RangePair>,
) -> Result<Response<Body>> {
    let mut uri = uri;
    loop {
        let data = data.clone().map(|d| Body::from_bytes(&d));
        let mut builder = Request::builder().method(method.clone()).uri(uri.clone());

        if let Some(RangePair { begin, end }) = range {
            builder = builder.header(header::RANGE, &format!("bytes={}-{}", begin, end));
        }
        let request = builder.body(data)?;
        let resp = client.send_async(request).await?;
        if resp.status().is_redirection() {
            if let Some(location) = resp.headers().get(header::LOCATION) {
                uri = location.to_str()?.parse()?;
                continue;
            } else {
                return Err(Error::NoLocation(format!("{}", uri)));
            }
        } else {
            is_success(&resp)?;
            return Ok(resp);
        }
    }
}
