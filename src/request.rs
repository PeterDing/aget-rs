use std::time::Duration;

use clap::crate_version;

use futures::{try_ready, Async, Future, Poll};

use actix::Addr;
use actix_web::{
    client::{ClientConnector, ClientRequest, ClientResponse},
    http, HttpMessage,
};

use http::{header, Method, Uri};

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

#[derive(Debug, Clone)]
pub struct AgetRequestOptions {
    uri: String,
    method: Method,
    headers: Vec<(String, String)>,
    body: Option<String>,
    concurrent: bool,
}

impl AgetRequestOptions {
    pub fn new(
        uri: &str,
        method: &str,
        headers: &[&str],
        body: Option<&str>,
    ) -> Result<AgetRequestOptions, AgetError> {
        let _method = match method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => return Err(AgetError::UnsupportedMethod),
        };

        let mut header_list = Vec::new();
        for header in headers.iter() {
            let (key, value) = parse_header(header)?;
            header_list.push((key.to_string(), value.to_string()));
        }

        Ok(AgetRequestOptions {
            method: _method,
            uri: uri.to_string(),
            headers: header_list,
            body: if let Some(body) = body {
                Some(body.to_string())
            } else {
                None
            },
            concurrent: true,
        })
    }

    pub fn build(
        &self,
        connector: Addr<ClientConnector>,
        timeout: u64,
    ) -> Result<ClientRequest, NetError> {
        let mut builder = ClientRequest::build();
        builder.with_connector(connector);
        builder
            .method(self.method.clone())
            .uri(self.uri.clone())
            .timeout(Duration::from_secs(timeout))
            .no_default_headers();

        for (ref key, ref val) in &self.headers {
            builder.header(key.as_str(), val.as_str());
        }

        // set header `Host`
        let uri = self.uri.parse::<Uri>()?;
        if let Some(host) = uri.host() {
            builder.set_header_if_none("Host", host);
        } else {
            return Err(NetError::InvaildUri(self.uri.to_string()));
        }

        // set user-agent if none
        let aget_ua = format!("aget/{}", crate_version!());
        builder.set_header_if_none("User-Agent", aget_ua);

        // set accept if none
        builder.set_header_if_none("Accept", "*/*");

        if let Some(ref body) = self.body {
            builder.body(body.clone())?;
        }
        let request = builder.finish()?;
        Ok(request)
    }

    pub fn uri(&self) -> String {
        self.uri.clone()
    }

    pub fn set_uri(&mut self, uri: &str) -> &mut Self {
        self.uri = uri.to_string();
        self
    }

    pub fn is_concurrent(&self) -> bool {
        self.concurrent
    }

    pub fn no_concurrency(&mut self) -> &mut Self {
        self.concurrent = false;
        self
    }
}

pub struct Redirect {
    options: AgetRequestOptions,
    connector: Addr<ClientConnector>,
    request: Option<Box<dyn Future<Item = Option<String>, Error = NetError>>>,
}

impl Redirect {
    pub fn new(options: AgetRequestOptions, connector: Addr<ClientConnector>) -> Redirect {
        Redirect {
            options,
            connector,
            request: None,
        }
    }
}

impl Future for Redirect {
    type Item = String;
    type Error = NetError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if let Some(ref mut request) = self.request {
                let r = try_ready!(request.poll());
                self.request = None;
                if let Some(new_uri) = r {
                    self.options.set_uri(&new_uri);
                } else {
                    return Ok(Async::Ready(self.options.uri()));
                }
            } else {
                let request = self.options.build(self.connector.clone(), 100);

                if let Err(err) = request {
                    return Err(err);
                }

                let mut request = request.unwrap();
                request
                    .headers_mut()
                    .insert(header::RANGE, "bytes=0-1".parse().unwrap());

                let request = request
                    .send()
                    .map_err(|err| {
                        print_err!("redirect error", err);
                        NetError::ActixError(format!("{}", err))
                    })
                    .and_then(|resp: ClientResponse| {
                        let status = resp.status();
                        if !(status.is_success() || status.is_redirection()) {
                            Err(NetError::Unsuccess(status.as_u16()))
                        } else {
                            Ok(resp)
                        }
                    })
                    .and_then(|resp: ClientResponse| {
                        debug!("Redirect response's headers", resp.headers());
                        // debug!("Redirect response's body", &resp.body().wait().unwrap());

                        if let Some(h) = resp.headers().get(header::LOCATION) {
                            if let Ok(s) = h.to_str() {
                                return Ok(Some(s.to_string()));
                            }
                        }
                        Ok(None)
                    });

                self.request = Some(Box::new(request));
            }
        }
    }
}

#[derive(Debug)]
pub enum ContentLengthItem {
    RangeLength(u64),
    DirectLength(u64),
    NoLength,
}

pub struct ContentLength {
    options: AgetRequestOptions,
    connector: Addr<ClientConnector>,
    request: Option<Box<dyn Future<Item = ContentLengthItem, Error = NetError>>>,
}

impl ContentLength {
    pub fn new(options: AgetRequestOptions, connector: Addr<ClientConnector>) -> ContentLength {
        ContentLength {
            options,
            connector,
            request: None,
        }
    }
}

impl Future for ContentLength {
    type Item = ContentLengthItem;
    type Error = NetError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if let Some(ref mut request) = self.request {
                let length = try_ready!(request.poll());
                self.request = None;
                return Ok(Async::Ready(length));
            } else {
                let request = self.options.build(self.connector.clone(), 100);

                if let Err(err) = request {
                    return Err(err);
                }

                let mut request = request.unwrap();
                request
                    .headers_mut()
                    .insert(header::RANGE, "bytes=0-1".parse().unwrap());

                let request = request
                    .send()
                    .map_err(|err| {
                        print_err!("content length request error", err);
                        NetError::ActixError(format!("{}", err))
                    })
                    .and_then(|resp: ClientResponse| {
                        if !resp.status().is_success() {
                            Err(NetError::Unsuccess(resp.status().as_u16()))
                        } else {
                            Ok(resp)
                        }
                    })
                    .and_then(|resp: ClientResponse| {
                        debug!("ContentLength response's headers", resp.headers());
                        // debug!("ContentLength response's body", &resp.body().wait().unwrap());

                        if let Some(h) = resp.headers().get(header::CONTENT_RANGE) {
                            if let Ok(s) = h.to_str() {
                                if let Some(index) = s.find("/") {
                                    if let Ok(length) = &s[index + 1..].parse::<u64>() {
                                        return Ok(ContentLengthItem::RangeLength(length.clone()));
                                    }
                                }
                            }
                        }

                        if let Some(h) = resp.headers().get(header::CONTENT_LENGTH) {
                            if let Ok(s) = h.to_str() {
                                if let Ok(length) = s.parse::<u64>() {
                                    return Ok(ContentLengthItem::DirectLength(length.clone()));
                                }
                            }
                        }

                        print_err!(
                            "server doesn't support partial requests",
                            "can't use range requests"
                        );

                        Ok(ContentLengthItem::NoLength)
                    });

                self.request = Some(Box::new(request));
            }
        }
    }
}
