use std::time::Duration;

use awc::{
    http::{header, Method, Uri},
    Client, ClientBuilder, ClientRequest, Connector,
};

use clap::crate_version;

use crate::error::{AgetError, NetError, Result};

fn parse_header(raw: &str) -> Result<(&str, &str), AgetError> {
    if let Some(index) = raw.find(": ") {
        return Ok((&raw[..index], &raw[index + 2..]));
    }
    if let Some(index) = raw.find(":") {
        return Ok((&raw[..index], &raw[index + 1..]));
    }
    Err(AgetError::HeaderParseError(raw.to_string()))
}

#[derive(Clone)]
pub struct AgetRequestOptions {
    uri: String,
    method: Method,
    headers: Vec<(String, String)>,
    body: Option<String>,
    concurrent: bool,
    client: Client,
}

impl AgetRequestOptions {
    pub fn new(
        uri: &str,
        method: &str,
        headers: &[&str],
        body: Option<&str>,
    ) -> Result<AgetRequestOptions, AgetError> {
        let method = match method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => return Err(AgetError::UnsupportedMethod),
        };

        let mut header_list = Vec::new();
        for header in headers.iter() {
            let (key, value) = parse_header(header)?;
            header_list.push((key.to_string(), value.to_string()));
        }

        let connector = Connector::new()
            .limit(0) // no limit simultaneous connections.
            .timeout(Duration::from_secs(5)) // DNS timeout
            .conn_keep_alive(Duration::from_secs(60))
            .conn_lifetime(Duration::from_secs(0))
            .finish();
        let client = ClientBuilder::new().connector(connector).finish();

        Ok(AgetRequestOptions {
            method,
            uri: uri.to_string(),
            headers: header_list,
            body: if let Some(body) = body {
                Some(body.to_string())
            } else {
                None
            },
            concurrent: true,
            client,
        })
    }

    pub fn build(&self) -> Result<ClientRequest, NetError> {
        // set user-agent if none
        let aget_ua = format!("aget/{}", crate_version!());

        let uri = self.uri.parse::<Uri>()?;
        let host = if let Some(host) = uri.host() {
            host
        } else {
            return Err(NetError::InvaildUri(self.uri.to_string()));
        };

        let mut client_request = self
            .client
            .request(self.method.clone(), self.uri.clone())
            .set_header_if_none("User-Agent", aget_ua) // set default user-agent
            .set_header_if_none("Accept", "*/*") // set accept if none
            .set_header_if_none("Host", host); // set header `Host`

        for (ref key, ref val) in &self.headers {
            client_request
                .headers_mut()
                .insert(key.as_str().parse()?, val.as_str().parse()?);
        }

        Ok(client_request)
    }

    pub fn uri(&self) -> String {
        self.uri.clone()
    }

    pub fn set_uri(&mut self, uri: &str) -> &mut Self {
        self.uri = uri.to_string();
        self
    }

    pub fn body(&self) -> &Option<String> {
        &self.body
    }

    pub fn is_concurrent(&self) -> bool {
        self.concurrent
    }

    pub fn no_concurrency(&mut self) -> &mut Self {
        self.concurrent = false;
        self
    }

    pub fn reset_connector(&mut self, timeout: u64, keep_alive: u64, lifetime: u64) -> &mut Self {
        let connector = Connector::new()
            .limit(0) // no limit simultaneous connections.
            .timeout(Duration::from_secs(timeout)) // DNS timeout
            .conn_keep_alive(Duration::from_secs(keep_alive))
            .conn_lifetime(Duration::from_secs(lifetime))
            .finish();
        self.client = ClientBuilder::new().connector(connector).finish();
        self
    }
}

// Get redirected uri and reset `AgetRequestOptions.uri`
pub async fn get_redirect_uri(options: &mut AgetRequestOptions) -> Result<(), NetError> {
    loop {
        let client_request = options.build()?;
        let resp = if let Some(body) = options.body() {
            client_request.send_body(body).await?
        } else {
            client_request.send().await?
        };
        let status = resp.status();
        if !(status.is_success() || status.is_redirection()) {
            return Err(NetError::Unsuccess(status.as_u16()));
        } else {
            if status.is_redirection() {
                if let Some(location) = resp.headers().get(header::LOCATION) {
                    options.set_uri(location.to_str()?);
                }
            }
            return Ok(());
        }
    }
}

#[derive(Debug)]
pub enum ContentLengthItem {
    RangeLength(u64),
    DirectLength(u64),
    NoLength,
}

pub async fn get_content_length(
    options: &mut AgetRequestOptions,
) -> Result<ContentLengthItem, NetError> {
    let client_request = options.build()?.header(header::RANGE, "bytes=0-1");
    let resp = if let Some(body) = options.body() {
        client_request.send_body(body).await?
    } else {
        client_request.send().await?
    };

    let status = resp.status();
    if !status.is_success() {
        return Err(NetError::Unsuccess(status.as_u16()));
    } else {
        if let Some(h) = resp.headers().get(header::CONTENT_RANGE) {
            if let Ok(s) = h.to_str() {
                if let Some(index) = s.find("/") {
                    if let Ok(length) = &s[index + 1..].parse::<u64>() {
                        return Ok(ContentLengthItem::RangeLength(length.clone()));
                    }
                }
            }
        } else {
            if let Some(h) = resp.headers().get(header::CONTENT_LENGTH) {
                if let Ok(s) = h.to_str() {
                    if let Ok(length) = s.parse::<u64>() {
                        return Ok(ContentLengthItem::DirectLength(length.clone()));
                    }
                }
            }
        }
    }
    Ok(ContentLengthItem::NoLength)
}
