use std::time::Duration;

use awc::http::header;

use futures::{channel::mpsc::Sender, stream::StreamExt};
use futures_util::sink::SinkExt;

use bytes::Bytes;

use crate::{
    chunk::{RangePart, RangeStack},
    error::NetError,
    request::AgetRequestOptions,
};

pub struct RequestTask {
    range_stack: RangeStack,
    sender: Sender<(RangePart, Bytes)>,
}

impl RequestTask {
    pub fn new(range_stack: RangeStack, sender: Sender<(RangePart, Bytes)>) -> RequestTask {
        RequestTask {
            range_stack,
            sender,
        }
    }

    fn pop_range(&mut self) -> Option<RangePart> {
        let mut stack = self.range_stack.borrow_mut();
        (*stack).pop()
    }

    fn push_range(&mut self, range: RangePart) {
        let mut stack = self.range_stack.borrow_mut();
        (*stack).push(range);
    }

    pub async fn run(&mut self, mut options: AgetRequestOptions) -> Result<(), NetError> {
        while let Some(range) = self.pop_range() {
            let timeout = if options.is_concurrent() {
                Duration::from_secs(60)
            } else {
                Duration::from_secs(10 * 24 * 60 * 60) // 10 days
            };

            let mut client_request = options.build()?.timeout(timeout);
            if options.is_concurrent() {
                client_request.headers_mut().insert(
                    header::RANGE,
                    format!("bytes={}-{}", range.start, range.end).parse()?,
                );
            }
            let resp = if let Some(body) = options.body() {
                client_request.send_body(body).await
            } else {
                client_request.send().await
            };

            if resp.is_err() {
                self.push_range(range);
                continue;
            }

            let mut resp = resp.unwrap();

            let status = resp.status();

            // handle redirect
            if status.is_redirection() {
                if let Some(location) = resp.headers().get(header::LOCATION) {
                    options.set_uri(location.to_str()?);
                }
                self.push_range(range);
                continue;
            }

            if !status.is_success() {
                debug!("request error", status);
                self.push_range(range);
                continue;
            }

            let mut range = range.clone();
            while let Some(chunk) = resp.next().await {
                if let Ok(chunk) = chunk {
                    let len = chunk.len() as u64;
                    let start = range.start;
                    let end = start + len;
                    range.start = end;

                    let mut sender = self.sender.clone();
                    // the sended RangePart is a close interval as header `Range`
                    sender.send((RangePart::new(start, end - 1), chunk)).await?;
                } else {
                    self.push_range(range);
                    break;
                }
            }
        }
        Ok(())
    }
}
