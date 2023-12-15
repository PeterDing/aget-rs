use std::time::Duration;

use crate::common::{
    errors::{Error, Result},
    net::{ContentLengthValue, HeaderMap, HeaderName, HttpClient, Method, Response, Url},
    range::RangePair,
};

pub fn parse_header(raw: &str) -> Result<(&str, &str), Error> {
    if let Some(index) = raw.find(": ") {
        return Ok((&raw[..index], &raw[index + 2..]));
    }
    if let Some(index) = raw.find(':') {
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
    timeout: Duration,
    dns_timeout: Duration,
    keep_alive: Duration,
) -> Result<HttpClient> {
    let mut default_headers = HeaderMap::new();
    headers.iter().for_each(|(k, v)| {
        default_headers.insert(k.parse::<HeaderName>().unwrap(), v.parse().unwrap());
    });
    if !default_headers.contains_key("accept") {
        default_headers.insert("accept", "*/*".parse().unwrap());
    }

    Ok(HttpClient::builder()
        .timeout(timeout)
        .connect_timeout(dns_timeout)
        .tcp_keepalive(keep_alive)
        .default_headers(default_headers)
        .build()?)
}

/// Check whether the response is success
/// Check if status is within 200-299.
pub fn is_success(resp: &reqwest::Response) -> Result<(), Error> {
    let status = resp.status();
    if !status.is_success() {
        Err(Error::Unsuccess(status.as_u16()))
    } else {
        Ok(())
    }
}

/// Send a request with a range header, returning the final url
pub async fn redirect(
    client: &HttpClient,
    method: Method,
    url: Url,
    data: Option<String>,
) -> Result<Url> {
    let mut req = client
        .request(method.clone(), url.clone())
        .header("range", "bytes=0-1");

    if let Some(d) = data {
        req = req.body(d);
    };

    let resp = req.send().await?;
    is_success(&resp)?; // Return unsuccess code

    Ok(resp.url().clone())
}

/// Get the content length of the resource
pub async fn redirect_and_contentlength(
    client: &HttpClient,
    method: Method,
    url: Url,
    data: Option<String>,
) -> Result<(Url, ContentLengthValue)> {
    let mut req = client
        .request(method.clone(), url.clone())
        .header("range", "bytes=0-1");
    if let Some(d) = data.clone() {
        req = req.body(d);
    }

    let resp = req.send().await?;
    is_success(&resp)?;

    let url = resp.url().clone();

    let status_code = resp.status();
    if status_code.as_u16() == 206 {
        let cl_str = resp
            .headers()
            .get("content-range")
            .unwrap()
            .to_str()
            .unwrap();
        let index = cl_str.find('/').unwrap();
        let length = cl_str[index + 1..].parse::<u64>()?;
        return Ok((url, ContentLengthValue::RangeLength(length)));
    } else {
        let content_length = resp.content_length();
        if let Some(length) = content_length {
            return Ok((url, ContentLengthValue::DirectLength(length)));
        } else {
            return Ok((url, ContentLengthValue::NoLength));
        }
    }
}

/// Send a request
pub async fn request(
    client: &HttpClient,
    method: Method,
    url: Url,
    data: Option<String>,
    range: Option<RangePair>,
) -> Result<Response> {
    let mut req = client.request(method, url);
    if let Some(RangePair { begin, end }) = range {
        req = req.header("range", format!("bytes={}-{}", begin, end));
    } else {
        req = req.header("range", "bytes=0-");
    }
    if let Some(d) = data.clone() {
        req = req.body(d);
    }

    let resp = req.send().await?;
    is_success(&resp)?;
    return Ok(resp);
}

pub fn join_url(base_url: &Url, url: &str) -> Result<Url> {
    let new_url: Url = if !url.to_lowercase().starts_with("http") {
        let base_url = Url::parse(&format!("{}", base_url))?;
        base_url.join(url)?.as_str().parse()?
    } else {
        url.parse()?
    };
    Ok(new_url)
}
