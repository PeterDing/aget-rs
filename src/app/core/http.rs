use std::{fmt, path::PathBuf, time::Duration};

use futures::{
    channel::mpsc::{channel, Sender},
    pin_mut, select, SinkExt, StreamExt,
};

use crate::{
    app::{
        receive::http_receiver::HttpReceiver,
        record::{common::RECORDER_FILE_SUFFIX, range_recorder::RangeRecorder},
    },
    common::{
        bytes::bytes_type::Bytes,
        errors::{Error, Result},
        file::File,
        net::{
            net::{build_http_client, redirect_and_contentlength, request},
            ContentLengthValue, HttpClient, Method, Url,
        },
        range::{split_pair, RangePair, SharedRangList},
        time::interval_stream,
    },
    features::{args::Args, running::Runnable, stack::StackLike},
};

/// Http task handler
pub struct HttpHandler<'a> {
    output: PathBuf,
    method: Method,
    url: Url,
    headers: Vec<(&'a str, &'a str)>,
    data: Option<&'a str>,
    concurrency: u64,
    chunk_size: u64,
    proxy: Option<&'a str>,
    timeout: Duration,
    client: HttpClient,
}

impl<'a> std::fmt::Debug for HttpHandler<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HttpHandler{{ method: {}, url: {}, headers: {:?}, data: {:?}, concurrency: {}, proxy: {:?} }}",
            self.method, self.url, self.headers, self.data, self.concurrency, self.proxy
        )
    }
}

impl<'a> HttpHandler<'a> {
    pub fn new(args: &(impl Args + std::fmt::Debug)) -> Result<HttpHandler> {
        let headers = args.headers();
        let timeout = args.timeout();
        let dns_timeout = args.dns_timeout();
        let keep_alive = args.keep_alive();
        let skip_verify_tls_cert = args.skip_verify_tls_cert();
        let proxy = args.proxy();

        let client = build_http_client(
            &headers,
            timeout,
            dns_timeout,
            keep_alive,
            skip_verify_tls_cert,
            proxy,
        )?;

        tracing::debug!("HttpHandler::new");

        Ok(HttpHandler {
            output: args.output(),
            method: args.method(),
            url: args.url(),
            headers,
            data: args.data(),
            concurrency: args.concurrency(),
            chunk_size: args.chunk_size(),
            proxy,
            timeout,
            client,
        })
    }

    async fn start(mut self) -> Result<()> {
        tracing::debug!("HttpHandler::start");

        // 0. Check whether task is completed
        tracing::debug!("HttpHandler: check whether task is completed");
        let mut rangerecorder =
            RangeRecorder::new(&*(self.output.to_string_lossy() + RECORDER_FILE_SUFFIX))?;
        if self.output.exists() && !rangerecorder.exists() {
            return Ok(());
        }

        // 1. redirect and get content_length
        tracing::debug!("HttpHandler: redirect and content_length start");
        let (url, cl) = redirect_and_contentlength(
            &self.client,
            self.method.clone(),
            self.url.clone(),
            self.data.map(|v| v.to_string()),
        )
        .await?;
        tracing::debug!("HttpHandler: redirect to: {}", url);
        tracing::debug!("HttpHandler: content_length: {:?}", cl);

        self.url = url;

        let content_length = {
            match cl {
                ContentLengthValue::DirectLength(l) => l,
                ContentLengthValue::RangeLength(l) => l,
                _ => 0,
            }
        };

        // 2. Compare recorded content length with the above one
        tracing::debug!("HttpHandler: compare recorded content length");
        let mut direct = true;
        if let ContentLengthValue::RangeLength(cl) = cl {
            if self.output.exists() {
                if rangerecorder.exists() {
                    rangerecorder.open()?;
                } else {
                    // Task is completed
                    return Ok(());
                }
            } else {
                // Init rangerecorder
                rangerecorder.remove().unwrap_or(()); // Missing error
                rangerecorder.open()?;
            }

            let pre_cl = rangerecorder.total()?;

            // Inital rangerecorder
            if pre_cl == 0 && pre_cl != cl {
                rangerecorder.write_total(cl)?;
                direct = false;
            }
            // Content is empty
            else if pre_cl == 0 && pre_cl == cl {
                File::new(&self.output, true)?.open()?;
                rangerecorder.remove()?;
                return Ok(());
            }
            // Content length is not consistent
            else if pre_cl != 0 && pre_cl != cl {
                return Err(Error::ContentLengthIsNotConsistent);
            }
            // Rewrite statistic status
            else if pre_cl != 0 && pre_cl == cl {
                rangerecorder.rewrite()?;
                direct = false;
            }
        }

        // 3. Create channel
        let (sender, receiver) = channel::<(RangePair, Bytes)>(self.concurrency as usize + 10);

        // 4. Dispatch Task
        tracing::debug!("HttpHandler: dispatch task: direct: {}", direct);
        if direct {
            // We need a new `HttpClient` which has unlimited life time for `DirectRequestTask`
            let mut task = DirectRequestTask::new(
                self.client.clone(),
                self.method.clone(),
                self.url.clone(),
                self.data.map(|v| v.to_string()),
                sender.clone(),
            );
            actix_rt::spawn(async move {
                task.start().await;
            });
        } else {
            // Make range pairs stack
            let mut stack = vec![];
            let gaps = rangerecorder.gaps()?;
            for gap in gaps.iter() {
                let mut list = split_pair(gap, self.chunk_size);
                stack.append(&mut list);
            }
            stack.reverse();
            let stack = SharedRangList::new(stack);
            // let stack = SharedRangList::new(rangerecorder.gaps()?);
            tracing::debug!("HttpHandler: range stack length: {}", stack.len());

            let concurrency = std::cmp::min(stack.len() as u64, self.concurrency);
            for i in 1..concurrency + 1 {
                let mut task = RangeRequestTask::new(
                    self.client.clone(),
                    self.method.clone(),
                    self.url.clone(),
                    self.data.map(|v| v.to_string()),
                    stack.clone(),
                    sender.clone(),
                    i,
                    self.timeout,
                );
                actix_rt::spawn(async move {
                    task.start().await;
                });
            }
        }
        drop(sender); // Remove the reference and let `Task` to handle it

        // 5. Create receiver
        tracing::debug!("HttpHandler: create receiver");
        let mut httpreceiver = HttpReceiver::new(&self.output, direct, content_length)?;
        httpreceiver.start(receiver).await?;

        // 6. Task succeeds. Remove rangerecorder file
        rangerecorder.remove().unwrap_or(()); // Missing error
        Ok(())
    }
}

impl<'a> Runnable for HttpHandler<'a> {
    fn run(self) -> Result<()> {
        let sys = actix_rt::System::new();
        sys.block_on(self.start())
    }
}

/// Directly request the resource without range header
struct DirectRequestTask {
    client: HttpClient,
    method: Method,
    url: Url,
    data: Option<String>,
    sender: Sender<(RangePair, Bytes)>,
}

impl DirectRequestTask {
    #[tracing::instrument(skip(client, sender))]
    fn new(
        client: HttpClient,
        method: Method,
        url: Url,
        data: Option<String>,
        sender: Sender<(RangePair, Bytes)>,
    ) -> DirectRequestTask {
        DirectRequestTask {
            client,
            method,
            url,
            data,
            sender,
        }
    }

    #[tracing::instrument(skip(self))]
    async fn start(&mut self) {
        loop {
            let resp = request(
                &self.client,
                self.method.clone(),
                self.url.clone(),
                self.data.clone(),
                None,
            )
            .await;
            if let Err(err) = resp {
                tracing::error!("DirectRequestTask request error: {:?}", err);
                continue;
            }
            let resp = resp.unwrap();
            let mut stream = resp.bytes_stream();

            let mut offset = 0u64;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => {
                        let len = chunk.len();
                        if len == 0 {
                            continue;
                        }

                        let pair = RangePair::new(offset, offset + len as u64 - 1); // The pair is a closed interval
                        self.sender.send((pair, chunk)).await.unwrap();
                        offset += len as u64;
                    }
                    Err(err) => {
                        tracing::error!("DirectRequestTask read error: {:?}", err);
                        break;
                    }
                }
            }

            break;
        }
    }
}

/// Request the resource with a range header which is in the `SharedRangList`
struct RangeRequestTask {
    client: HttpClient,
    method: Method,
    url: Url,
    data: Option<String>,
    stack: SharedRangList,
    sender: Sender<(RangePair, Bytes)>,
    id: u64,
    timeout: Duration,
}

impl RangeRequestTask {
    #[tracing::instrument(skip(client, sender))]
    fn new(
        client: HttpClient,
        method: Method,
        url: Url,
        data: Option<String>,
        stack: SharedRangList,
        sender: Sender<(RangePair, Bytes)>,
        id: u64,
        timeout: Duration,
    ) -> RangeRequestTask {
        RangeRequestTask {
            client,
            method,
            url,
            data,
            stack,
            sender,
            id,
            timeout,
        }
    }

    #[tracing::instrument(skip(self))]
    async fn start(&mut self) {
        tracing::debug!("Fire RangeRequestTask: {}", self.id);
        while let Some(pair) = self.stack.pop() {
            match self.req(pair).await {
                // Exit whole process when `Error::InnerError` is returned
                Err(Error::InnerError(msg)) => {
                    tracing::error!("RangeRequestTask {}: InnerError: {}", self.id, msg);
                    actix_rt::System::current().stop();
                }
                Err(err @ Error::Timeout) => {
                    tracing::debug!("RangeRequestTask timeout: {}", err); // Missing Timeout at runtime
                }
                Err(err) => {
                    tracing::debug!("RangeRequestTask {}: error: {}", self.id, err);
                }
                _ => {}
            }
        }
    }

    async fn req(&mut self, pair: RangePair) -> Result<()> {
        let resp = request(
            &self.client,
            self.method.clone(),
            self.url.clone(),
            self.data.clone(),
            Some(pair),
        )
        .await;

        if let Err(err) = resp {
            self.stack.push(pair);
            return Err(err);
        }
        let resp = resp.unwrap();

        let RangePair { begin, end } = pair;
        let length = pair.length();
        let mut count = 0u64;
        let mut offset = begin;

        let stream = resp.bytes_stream().fuse();

        // Set timeout for reading
        let tick = interval_stream(self.timeout).fuse();

        pin_mut!(stream, tick);
        let mut fire = false;
        loop {
            select! {
                item = stream.next() => {
                    if let Some(item) = item {
                        match item {
                            Ok(chunk) => {
                                let len = chunk.len();
                                if len == 0 {
                                    continue;
                                }

                                // The pair is a closed interval
                                let pr = RangePair::new(offset, offset + len as u64 - 1);
                                if let Err(err) = self.sender.send((pr, chunk)).await {
                                    let pr = RangePair::new(offset, end);
                                    self.stack.push(pr);
                                    return Err(Error::InnerError(format!(
                                        "Error at `http::RangeRequestTask`: Sender error: {:?}",
                                        err
                                    )));
                                }
                                offset += len as u64;
                                count += len as u64;
                            }
                            Err(err) => {
                                let pr = RangePair::new(offset, end);
                                self.stack.push(pr);
                                return Err(err.into());
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ = tick.next() => {
                    if fire {
                        let pr = RangePair::new(offset, end);
                        self.stack.push(pr);
                        return Err(Error::Timeout);
                    } else {
                        fire = true;
                    }
                }
            }
        }

        // Check range length whether is equal to the length of all received chunk
        if count != length {
            let pr = RangePair::new(offset, end);
            self.stack.push(pr);
            Err(Error::UncompletedRead)
        } else {
            Ok(())
        }
    }
}
