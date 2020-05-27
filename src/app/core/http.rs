use std::{path::PathBuf, time::Duration};

use futures::{
    channel::mpsc::{channel, Sender},
    select,
    stream::StreamExt,
    SinkExt,
};

use actix_rt::{spawn, time::interval, System};

use crate::{
    app::{
        receive::http_receiver::HttpReceiver,
        record::{common::RECORDER_FILE_SUFFIX, range_recorder::RangeRecorder},
    },
    common::{
        bytes::bytes_type::{Buf, Bytes},
        errors::{Error, Result},
        file::File,
        net::{
            net::{build_http_client, redirect_and_contentlength, request},
            ConnectorConfig, ContentLengthValue, HttpClient, Method, Uri,
        },
        range::{split_pair, RangePair, SharedRangList},
    },
    features::{args::Args, running::Runnable, stack::StackLike},
};

/// Http task handler
pub struct HttpHandler {
    output: PathBuf,
    method: Method,
    uri: Uri,
    headers: Vec<(String, String)>,
    data: Option<Bytes>,
    connector_config: ConnectorConfig,
    concurrency: u64,
    chunk_size: u64,
    retries: u64,
    retry_wait: u64,
    // proxy is None, because `awc` does not suppurt proxy
    proxy: Option<String>,
    client: HttpClient,
}

impl HttpHandler {
    pub fn new(args: &impl Args) -> Result<HttpHandler> {
        let headers = args.headers();
        let timeout = args.timeout();
        let dns_timeout = args.dns_timeout();
        let keep_alive = args.keep_alive();
        let lifetime = args.lifetime();

        let connector_config = ConnectorConfig {
            timeout,
            dns_timeout,
            keep_alive,
            lifetime,
            disable_redirects: true,
        };

        let hds: Vec<(&str, &str)> = headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let client = build_http_client(
            hds.as_ref(),
            timeout,
            dns_timeout,
            keep_alive,
            lifetime,
            true, // Disable rediect
        );

        debug!("HttpHandler::new");

        Ok(HttpHandler {
            output: args.output(),
            method: args.method(),
            uri: args.uri(),
            headers,
            data: args.data().map(|ref mut d| d.to_bytes()),
            connector_config,
            concurrency: args.concurrency(),
            chunk_size: args.chunk_size(),
            retries: args.retries(),
            retry_wait: args.retry_wait(),
            proxy: None,
            client,
        })
    }

    async fn start(mut self) -> Result<()> {
        debug!("HttpHandler::start");

        // 0. Check whether task is completed
        debug!("HttpHandler: check whether task is completed");
        let mut rangerecorder =
            RangeRecorder::new(&*(self.output.to_string_lossy() + RECORDER_FILE_SUFFIX))?;
        if self.output.exists() && !rangerecorder.exists() {
            return Ok(());
        }

        // 1. redirect and get content_length
        debug!("HttpHandler: redirect and content_length start");
        let (uri, cl) = redirect_and_contentlength(
            &self.client,
            self.method.clone(),
            self.uri.clone(),
            self.data.clone(),
        )
        .await?;
        debug!("HttpHandler: redirect to", uri);
        debug!("HttpHandler: content_length", cl);

        self.uri = uri;

        // 2. Compare recorded content length with the above one
        debug!("HttpHandler: compare recorded content length");
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
        debug!("HttpHandler: dispatch task: direct", direct);
        if direct {
            // We need a new `HttpClient` which has unlimited life time for `DirectRequestTask`
            let hds: Vec<(&str, &str)> = self
                .headers
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let ConnectorConfig { dns_timeout, .. } = self.connector_config;
            let client = build_http_client(
                hds.as_ref(),
                Duration::from_secs(60), // timeout for waiting the begin of response
                dns_timeout,             // dns timeout
                Duration::from_secs(60), // keep alive
                Duration::from_secs(0),  // lifetime
                true,                    // Disable rediect
            );
            let mut task = DirectRequestTask::new(
                client,
                self.method.clone(),
                self.uri.clone(),
                self.data.clone(),
                sender.clone(),
            );
            spawn(async move {
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
            debug!("HttpHandler: range stack length", stack.len());

            let concurrency = std::cmp::min(stack.len() as u64, self.concurrency);
            for i in 1..concurrency + 1 {
                let mut task = RangeRequestTask::new(
                    self.client.clone(),
                    self.method.clone(),
                    self.uri.clone(),
                    self.data.clone(),
                    stack.clone(),
                    sender.clone(),
                    i,
                    self.connector_config.timeout,
                );
                spawn(async move {
                    task.start().await;
                });
            }
        }
        drop(sender); // Remove the reference and let `Task` to handle it

        // 5. Create receiver
        debug!("HttpHandler: create receiver");
        let mut httpreceiver = HttpReceiver::new(&self.output, direct)?;
        httpreceiver.start(receiver).await?;

        // 6. Task succeeds. Remove rangerecorder file
        rangerecorder.remove().unwrap_or(()); // Missing error
        Ok(())
    }
}

impl Runnable for HttpHandler {
    fn run(self) -> Result<()> {
        let mut sys = System::new("HttpHandler");
        sys.block_on(self.start())
    }
}

/// Directly request the resource without range header
struct DirectRequestTask {
    client: HttpClient,
    method: Method,
    uri: Uri,
    data: Option<Bytes>,
    sender: Sender<(RangePair, Bytes)>,
}

impl DirectRequestTask {
    fn new(
        client: HttpClient,
        method: Method,
        uri: Uri,
        data: Option<Bytes>,
        sender: Sender<(RangePair, Bytes)>,
    ) -> DirectRequestTask {
        DirectRequestTask {
            client,
            method,
            uri,
            data,
            sender,
        }
    }

    async fn start(&mut self) {
        loop {
            let resp = request(
                &self.client,
                self.method.clone(),
                self.uri.clone(),
                self.data.clone(),
                None,
            )
            .await;
            if let Err(err) = resp {
                print_err!("DirectRequestTask request error", err);
                continue;
            }
            let mut resp = resp.unwrap();

            let mut offset = 0u64;
            while let Some(item) = resp.next().await {
                match item {
                    Ok(chunk) => {
                        let len = chunk.len();
                        if len == 0 {
                            continue;
                        }

                        let pair = RangePair::new(offset, offset + len as u64 - 1); // The pair is a closed interval
                        if self.sender.send((pair, chunk)).await.is_err() {
                            break;
                        }
                        offset += len as u64;
                    }
                    Err(err) => {
                        print_err!("DirectRequestTask read error", err);
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
    uri: Uri,
    data: Option<Bytes>,
    stack: SharedRangList,
    sender: Sender<(RangePair, Bytes)>,
    id: u64,
    timeout: Duration,
}

impl RangeRequestTask {
    fn new(
        client: HttpClient,
        method: Method,
        uri: Uri,
        data: Option<Bytes>,
        stack: SharedRangList,
        sender: Sender<(RangePair, Bytes)>,
        id: u64,
        timeout: Duration,
    ) -> RangeRequestTask {
        RangeRequestTask {
            client,
            method,
            uri,
            data,
            stack,
            sender,
            id,
            timeout,
        }
    }

    async fn start(&mut self) {
        debug!("Fire RangeRequestTask", self.id);
        while let Some(pair) = self.stack.pop() {
            match self.req(pair).await {
                // Exit whole process when `Error::InnerError` is returned
                Err(Error::InnerError(msg)) => {
                    print_err!(format!("RangeRequestTask {}: InnerError", self.id), msg);
                    System::current().stop();
                }
                Err(err) => {
                    print_err!(format!("RangeRequestTask {}: error", self.id), err);
                }
                _ => {}
            }
        }
    }

    async fn req(&mut self, pair: RangePair) -> Result<()> {
        let resp = request(
            &self.client,
            self.method.clone(),
            self.uri.clone(),
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

        // Set timeout for reading
        let mut resp = resp.fuse();
        let mut tick = interval(self.timeout).fuse();
        let mut fire = false;
        loop {
            select! {
                item = resp.next() => {
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
