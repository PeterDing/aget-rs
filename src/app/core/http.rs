use std::{path::PathBuf, sync::Arc};

use async_std::{io::ReadExt, process::exit, task as std_task};

use futures::{
    channel::mpsc::{channel, Sender},
    SinkExt,
};

use crate::{
    app::{
        receive::http_receiver::HttpReceiver,
        record::{common::RECORDER_FILE_SUFFIX, range_recorder::RangeRecorder},
    },
    common::{
        buf::SIZE,
        bytes::bytes_type::{Buf, Bytes, BytesMut},
        errors::{Error, Result},
        file::File,
        net::{
            net::{build_http_client, content_length, redirect, request},
            net_type::{ContentLengthValue, HttpClient, Method, Uri},
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
    timeout: u64,
    concurrency: u64,
    chunk_size: u64,
    retries: u64,
    retry_wait: u64,
    proxy: Option<String>,
    client: Arc<HttpClient>,
}

impl HttpHandler {
    pub fn new(args: &impl Args) -> Result<HttpHandler> {
        let headers = args.headers();
        let timeout = args.timeout();
        let proxy = args.proxy();

        let hds: Vec<(&str, &str)> = headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let client = build_http_client(hds.as_ref(), timeout, proxy.as_deref())?;

        debug!("HttpHandler::new");

        Ok(HttpHandler {
            output: args.output(),
            method: args.method(),
            uri: args.uri(),
            headers,
            data: args.data().map(|ref mut d| d.to_bytes()),
            timeout,
            concurrency: args.concurrency(),
            chunk_size: args.chunk_size(),
            retries: args.retries(),
            retry_wait: args.retry_wait(),
            proxy,
            client: Arc::new(client),
        })
    }

    async fn start(&mut self) -> Result<()> {
        debug!("HttpHandler::start");

        // 0. Check whether task is completed
        debug!("HttpHandler: check whether task is completed");
        let mut rangerecorder =
            RangeRecorder::new(&*(self.output.to_string_lossy() + RECORDER_FILE_SUFFIX))?;
        if self.output.exists() && !rangerecorder.exists() {
            return Ok(());
        }

        // 1. Redirect
        debug!("HttpHandler: redirect start");
        let uri = redirect(
            &self.client,
            self.method.clone(),
            self.uri.clone(),
            self.data.clone(),
        )
        .await?;
        debug!("HttpHandler: redirect to", uri);
        self.uri = uri;

        // 2. get content_length
        debug!("HttpHandler: content_length start");
        let cl = content_length(
            &self.client,
            self.method.clone(),
            self.uri.clone(),
            self.data.clone(),
        )
        .await?;
        debug!("HttpHandler: content_length", cl);

        // 3. Compare recorded content length with the above one
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

        // 4. Create channel
        let (sender, receiver) = channel::<(RangePair, Bytes)>(self.concurrency as usize + 10);

        // 5. Dispatch Task
        debug!("HttpHandler: dispatch task: direct", direct);
        if direct {
            let mut task = DirectRequestTask::new(
                self.client.clone(),
                self.method.clone(),
                self.uri.clone(),
                self.data.clone(),
                sender.clone(),
            );
            std_task::spawn(async move {
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
                );
                std_task::spawn(async move {
                    task.start().await;
                });
            }
        }
        drop(sender); // Remove the reference and let `Task` to handle it

        // 6. Create receiver
        debug!("HttpHandler: create receiver");
        let mut httpreceiver = HttpReceiver::new(&self.output, direct)?;
        httpreceiver.start(receiver).await?;

        // 7. Task succeeds. Remove rangerecorder file
        rangerecorder.remove().unwrap_or(()); // Missing error
        Ok(())
    }
}

impl Runnable for HttpHandler {
    fn run(&mut self) -> Result<()> {
        std_task::block_on(self.start())
    }
}

/// Directly request the resource without range header
struct DirectRequestTask {
    client: Arc<HttpClient>,
    method: Method,
    uri: Uri,
    data: Option<Bytes>,
    sender: Sender<(RangePair, Bytes)>,
}

impl DirectRequestTask {
    fn new(
        client: Arc<HttpClient>,
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
                &*self.client,
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
            let resp = resp.unwrap();

            let mut buf = [0; SIZE];
            let mut offset = 0;
            let mut reader = resp.into_body();
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => {
                        return;
                    }
                    Ok(len) => {
                        let pair = RangePair::new(offset, offset + len as u64 - 1); // The pair is a closed interval
                        let mut b = BytesMut::from(&buf[..len]);
                        if self.sender.send((pair, b.to_bytes())).await.is_err() {
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
        }
    }
}

/// Request the resource with a range header which is in the `SharedRangList`
struct RangeRequestTask {
    client: Arc<HttpClient>,
    method: Method,
    uri: Uri,
    data: Option<Bytes>,
    stack: SharedRangList,
    sender: Sender<(RangePair, Bytes)>,
    id: u64,
}

impl RangeRequestTask {
    fn new(
        client: Arc<HttpClient>,
        method: Method,
        uri: Uri,
        data: Option<Bytes>,
        stack: SharedRangList,
        sender: Sender<(RangePair, Bytes)>,
        id: u64,
    ) -> RangeRequestTask {
        RangeRequestTask {
            client,
            method,
            uri,
            data,
            stack,
            sender,
            id,
        }
    }

    async fn start(&mut self) {
        while let Some(pair) = self.stack.pop() {
            match self.req(pair).await {
                // Exit whole process when `Error::InnerError` is returned
                Err(Error::InnerError(msg)) => {
                    print_err!(format!("RangeRequestTask {}: InnerError", self.id), msg);
                    exit(1);
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
            &*self.client,
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

        let length = pair.length();
        let mut count = 0;
        let mut buf = [0; SIZE];
        let mut offset = pair.begin;
        let mut reader = resp.into_body();
        loop {
            // Reads some bytes from the byte stream.
            //
            // Returns the number of bytes read from the start of the buffer.
            //
            // If the return value is `Ok(n)`, then it must be guaranteed that
            // `0 <= n <= buf.len()`. A nonzero `n` value indicates that the buffer has been
            // filled in with `n` bytes of data. If `n` is `0`, then it can indicate one of two
            // scenarios:
            //
            // 1. This reader has reached its "end of file" and will likely no longer be able to
            //    produce bytes. Note that this does not mean that the reader will always no
            //    longer be able to produce bytes.
            // 2. The buffer specified was 0 bytes in length.
            match reader.read(&mut buf).await {
                Ok(0) => {
                    if count != length {
                        let pr = RangePair::new(offset, pair.end);
                        self.stack.push(pr);
                        return Err(Error::UncompletedRead);
                    } else {
                        return Ok(());
                    }
                }
                Ok(len) => {
                    let pr = RangePair::new(offset, offset + len as u64 - 1); // The pair is a closed interval
                    let mut b = BytesMut::from(&buf[..len]);
                    if let Err(err) = self.sender.send((pr, b.to_bytes())).await {
                        let pr = RangePair::new(offset, pair.end);
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
                    let pr = RangePair::new(offset, pair.end);
                    self.stack.push(pr);
                    return Err(err.into());
                }
            }
        }
    }
}
