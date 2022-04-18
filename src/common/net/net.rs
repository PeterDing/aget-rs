use std::time::Duration;

use crate::common::{
    errors::{Error, Result},
    net::{
        header, ClientResponse, Connector, ContentLengthValue, HttpClient, Method, RClientResponse,
        Uri, Url,
    },
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
    lifetime: Duration,
    disable_redirects: bool,
) -> HttpClient {
    let conn = Connector::new()
        // Set total number of simultaneous connections per type of scheme.
        //
        // If limit is 0, the connector has no limit.
        // The default limit size is 100.
        .limit(0)
        // Connection timeout
        //
        // i.e. max time to connect to remote host including dns name resolution.
        // Set to 1 second by default.
        .timeout(dns_timeout)
        // Set keep-alive period for opened connection.
        //
        // Keep-alive period is the period between connection usage. If
        // the delay between repeated usages of the same connection
        // exceeds this period, the connection is closed.
        // Default keep-alive period is 15 seconds.
        .conn_keep_alive(keep_alive)
        // Set max lifetime period for connection.
        //
        // Connection lifetime is max lifetime of any opened connection
        // until it is closed regardless of keep-alive period.
        // Default lifetime period is 75 seconds.
        .conn_lifetime(lifetime);

    let mut builder = HttpClient::builder()
        .connector(conn)
        // Set request timeout
        //
        // Request timeout is the total time before a response must be received.
        // Default value is 5 seconds.
        .timeout(timeout)
        // Here we do not use default headers.
        .no_default_headers();

    if disable_redirects {
        builder = builder.disable_redirects();
    }

    // Add Default headers
    for (k, v) in headers {
        builder = builder.add_default_header((*k, *v));
    }

    builder.finish()
}

/// Check whether the response is success
/// Check if status is within 200-299.
pub fn is_success<T>(resp: &ClientResponse<T>) -> Result<(), Error> {
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
    data: Option<String>,
) -> Result<Uri> {
    let mut uri = uri;
    loop {
        let req = client
            .request(method.clone(), uri.clone())
            .insert_header_if_none((header::ACCEPT, "*/*")) // set accept if none
            .insert_header((header::RANGE, "bytes=0-1"));

        let resp = if let Some(d) = data.clone() {
            req.send_body(d).await?
        } else {
            req.send().await?
        };

        if !resp.status().is_redirection() {
            is_success(&resp)?; // Return unsuccess code
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
pub async fn redirect_and_contentlength(
    client: &HttpClient,
    method: Method,
    uri: Uri,
    data: Option<String>,
) -> Result<(Uri, ContentLengthValue)> {
    let mut uri = uri;
    loop {
        let req = client
            .request(method.clone(), uri.clone())
            .insert_header_if_none((header::ACCEPT, "*/*")) // set accept if none
            .insert_header((header::RANGE, "bytes=0-1"));

        let resp = if let Some(d) = data.clone() {
            req.send_body(d).await?
        } else {
            req.send().await?
        };

        let headers = resp.headers();
        if resp.status().is_redirection() {
            if let Some(location) = headers.get(header::LOCATION) {
                let uri_str = location.to_str()?;
                uri = join_uri(&uri, uri_str)?;
                continue;
            } else {
                return Err(Error::NoLocation(format!("{}", uri)));
            }
        } else {
            is_success(&resp)?;
            if let Some(h) = headers.get(header::CONTENT_RANGE) {
                if let Ok(s) = h.to_str() {
                    if let Some(index) = s.find('/') {
                        if let Ok(length) = s[index + 1..].parse::<u64>() {
                            return Ok((uri, ContentLengthValue::RangeLength(length)));
                        }
                    }
                }
            }
            if let Some(h) = resp.headers().get(header::CONTENT_LENGTH) {
                if let Ok(s) = h.to_str() {
                    if let Ok(length) = s.parse::<u64>() {
                        return Ok((uri, ContentLengthValue::DirectLength(length)));
                    }
                }
            }
            break;
        }
    }
    Ok((uri, ContentLengthValue::NoLength))
}

/// Send a request
pub async fn request(
    client: &HttpClient,
    method: Method,
    uri: Uri,
    data: Option<String>,
    range: Option<RangePair>,
) -> Result<RClientResponse> {
    let mut uri = uri;
    loop {
        let mut req = client
            .request(method.clone(), uri.clone())
            .insert_header_if_none((header::ACCEPT, "*/*")); // set accept if none

        if let Some(RangePair { begin, end }) = range {
            req = req.insert_header((header::RANGE, format!("bytes={}-{}", begin, end)));
        } else {
            req = req.insert_header((header::RANGE, String::from("bytes=0-")));
        }

        let resp = if let Some(d) = data.clone() {
            req.send_body(d).await?
        } else {
            req.send().await?
        };

        if resp.status().is_redirection() {
            if let Some(location) = resp.headers().get(header::LOCATION) {
                let uri_str = location.to_str()?;
                uri = join_uri(&uri, uri_str)?;
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

pub fn join_uri(base_uri: &Uri, uri: &str) -> Result<Uri> {
    let new_uri: Uri = if !uri.to_lowercase().starts_with("http") {
        let base_url = Url::parse(&format!("{}", base_uri))?;
        base_url.join(uri)?.as_str().parse()?
    } else {
        uri.parse()?
    };
    Ok(new_uri)
}
