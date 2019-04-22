use std::sync::{Arc, Mutex};

use actix::Addr;
use actix_web::client::{ClientConnector, ClientResponse};
use actix_web::{http, HttpMessage};

use futures::sync::mpsc::Sender;
use futures::{Async, Future, Poll, Sink, Stream};

use http::header;

use bytes::Bytes;

use crate::chunk::{AtomicRangStack, RangePart, RangeStack};
use crate::error::NetError;
use crate::request::AgetRequestOptions;

pub struct RequestTask {
    options: AgetRequestOptions,
    connector: Addr<ClientConnector>,
    request: Option<Box<dyn Future<Item = (), Error = NetError>>>,
    range_stack: RangeStack,
    range: AtomicRangStack,
    sender: Sender<(RangePart, Bytes)>,
}

impl RequestTask {
    pub fn new(
        range_stack: RangeStack,
        options: AgetRequestOptions,
        connector: Addr<ClientConnector>,
        sender: Sender<(RangePart, Bytes)>,
    ) -> RequestTask {
        RequestTask {
            options,
            connector,
            request: None,
            range_stack,
            range: Arc::new(Mutex::new(RangePart::new(0, 0))),
            sender,
        }
    }

    fn pop_range(&mut self) -> Option<RangePart> {
        let mut stack = self.range_stack.lock().unwrap();
        (*stack).pop()
    }

    fn push_range(&mut self, range: RangePart) {
        let mut stack = self.range_stack.lock().unwrap();
        (*stack).push(range);
    }
}

impl Future for RequestTask {
    type Item = ();
    type Error = NetError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if let Some(ref mut request) = self.request {
                match request.poll() {
                    Ok(Async::Ready(t)) => (),
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(err) => {
                        debug!("request error", err);
                        let range = self.range.clone();
                        let range = range.lock().unwrap();
                        self.push_range(range.clone());
                    }
                }
                self.request = None;
            } else {
                if let Some(range) = self.pop_range() {
                    let request = self.options.build(self.connector.clone(), 5);

                    if let Err(err) = request {
                        print_err!("build request fails", err);
                        return Err(err);
                    }

                    let mut request = request.unwrap();
                    request.headers_mut().insert(
                        header::RANGE,
                        format!("bytes={}-{}", range.start, range.end)
                            .parse()
                            .unwrap(),
                    );

                    self.range = Arc::new(Mutex::new(range));
                    let range = self.range.clone();

                    let sender = self.sender.clone();

                    let request = request
                        .send()
                        .map_err(|err| {
                            // print_err!("request fails", err);
                            // debug!(format!("request error: {:?}", err));
                            NetError::ActixError(format!("{}", err))
                        })
                        .and_then(|resp| {
                            if !resp.status().is_success() {
                                Err(NetError::Unsuccess(resp.status().as_u16()))
                            } else {
                                Ok(resp)
                            }
                        })
                        .from_err()
                        .and_then(move |resp: ClientResponse| {
                            resp.payload().from_err().fold(
                                sender.clone(),
                                move |_, chunk| {
                                    let len = chunk.len() as u64;
                                    let mut range = range.lock().unwrap();
                                    let start = range.start;
                                    let end = start + len;
                                    range.start = end;

                                    sender
                                        .clone()
                                        // `send` takes self
                                        //
                                        // the sended RangePart is a close interval as header
                                        // `Range`
                                        .send((RangePart::new(start, end - 1), chunk))
                                        .map_err(|err| {
                                            print_err!("sender fails", err);
                                            NetError::ActixError(format!("{}", err))
                                        })
                                    // Ok::<_, NetError>(())
                                },
                            )
                        })
                        .map(|_| ());

                    self.request = Some(Box::new(request));
                } else {
                    return Ok(Async::Ready(()));
                }
            }
        }
    }
}
