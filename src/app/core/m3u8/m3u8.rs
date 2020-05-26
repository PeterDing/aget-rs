use std::{cell::Cell, path::PathBuf, rc::Rc, time::Duration};

use futures::{
    channel::mpsc::{channel, Sender},
    select,
    stream::StreamExt,
    SinkExt,
};

use actix_rt::{
    spawn,
    time::{delay_for, interval},
    System,
};

use crate::{
    app::{
        core::m3u8::common::{get_m3u8, M3u8Segment, SharedM3u8SegmentList},
        receive::m3u8_receiver::M3u8Receiver,
        record::{bytearray_recorder::ByteArrayRecorder, common::RECORDER_FILE_SUFFIX},
    },
    common::{
        bytes::bytes_type::{Buf, Bytes},
        crypto::decrypt_aes128,
        errors::{Error, Result},
        net::{
            net::{build_http_client, request},
            ConnectorConfig, HttpClient, Method, Uri,
        },
    },
    features::{args::Args, running::Runnable, stack::StackLike},
};

/// M3u8 task handler
pub struct M3u8Handler {
    output: PathBuf,
    method: Method,
    uri: Uri,
    headers: Vec<(String, String)>,
    data: Option<Bytes>,
    connector_config: ConnectorConfig,
    concurrency: u64,
    proxy: Option<String>,
    client: HttpClient,
}

impl M3u8Handler {
    pub fn new(args: &impl Args) -> Result<M3u8Handler> {
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

        debug!("M3u8Handler::new");

        Ok(M3u8Handler {
            output: args.output(),
            method: args.method(),
            uri: args.uri(),
            headers,
            data: args.data().map(|ref mut d| d.to_bytes()),
            connector_config,
            concurrency: args.concurrency(),
            proxy: None,
            client,
        })
    }

    async fn start(self) -> Result<()> {
        debug!("M3u8Handler::start");

        // 0. Check whether task is completed
        debug!("M3u8Handler: check whether task is completed");
        let mut bytearrayrecorder =
            ByteArrayRecorder::new(&*(self.output.to_string_lossy() + RECORDER_FILE_SUFFIX))?;
        if self.output.exists() && !bytearrayrecorder.exists() {
            return Ok(());
        }

        // 1. Get m3u8 info
        debug!("M3u8Handler: get m3u8");
        let mut ls = get_m3u8(
            &self.client,
            self.method.clone(),
            self.uri.clone(),
            self.data.clone(),
        )
        .await?;
        ls.reverse();

        // 2. Check recorder status
        if bytearrayrecorder.exists() {
            bytearrayrecorder.open()?;
            let total = bytearrayrecorder.index(0)?;
            if total != ls.len() as u64 {
                return Err(Error::PartsAreNotConsistent);
            } else {
                let index = bytearrayrecorder.index(1)?;
                ls.truncate((total - index) as usize);
            }
        } else {
            bytearrayrecorder.open()?;
            // Write total
            bytearrayrecorder.write(0, ls.len() as u64)?;
        }

        // Use atomic u64 to control the order of sending segment content
        let index = ls.last().unwrap().index;
        let sharedindex = Rc::new(Cell::new(index));
        let stack = SharedM3u8SegmentList::new(ls);
        debug!("M3u8Handler: segments", stack.len());

        // 3. Create channel
        let (sender, receiver) = channel::<(u64, Bytes)>(self.concurrency as usize + 10);

        // 4. Spawn request task
        let concurrency = std::cmp::min(stack.len() as u64, self.concurrency);
        for i in 1..concurrency + 1 {
            let mut task = RequestTask::new(
                self.client.clone(),
                stack.clone(),
                sender.clone(),
                i,
                sharedindex.clone(),
                self.connector_config.timeout,
            );
            spawn(async move {
                task.start().await;
            });
        }
        drop(sender); // Remove the reference and let `Task` to handle it

        // 5. Create receiver
        debug!("M3u8Handler: create receiver");
        let mut m3u8receiver = M3u8Receiver::new(&self.output)?;
        m3u8receiver.start(receiver).await?;

        // 6. Task succeeds. Remove `ByteArrayRecorder` file
        bytearrayrecorder.remove().unwrap_or(()); // Missing error
        Ok(())
    }
}

impl Runnable for M3u8Handler {
    fn run(self) -> Result<()> {
        let mut sys = System::new("M3u8Handler");
        sys.block_on(self.start())
    }
}

/// Request the resource with a range header which is in the `SharedRangList`
struct RequestTask {
    client: HttpClient,
    stack: SharedM3u8SegmentList,
    sender: Sender<(u64, Bytes)>,
    id: u64,
    shared_index: Rc<Cell<u64>>,
    timeout: Duration,
}

impl RequestTask {
    fn new(
        client: HttpClient,
        stack: SharedM3u8SegmentList,
        sender: Sender<(u64, Bytes)>,
        id: u64,
        sharedindex: Rc<Cell<u64>>,
        timeout: Duration,
    ) -> RequestTask {
        RequestTask {
            client,
            stack,
            sender,
            id,
            shared_index: sharedindex,
            timeout,
        }
    }

    async fn start(&mut self) {
        debug!("Fire RequestTask", self.id);
        while let Some(segment) = self.stack.pop() {
            loop {
                match self.req(segment.clone()).await {
                    // Exit whole process when `Error::InnerError` is returned
                    Err(Error::InnerError(msg)) => {
                        print_err!(format!("RequestTask {}: InnerError", self.id), msg);
                        System::current().stop();
                    }
                    Err(err) => {
                        print_err!(format!("RequestTask {}: error", self.id), err);
                        delay_for(Duration::from_secs(1)).await;
                    }
                    _ => break,
                }
            }
        }
    }

    async fn req(&mut self, segment: M3u8Segment) -> Result<()> {
        let resp = request(
            &self.client,
            segment.method.clone(),
            segment.uri.clone(),
            segment.data.clone(),
            None,
        )
        .await?;

        let index = segment.index;

        // !!! resp.body().await can be overflow
        let mut buf: Vec<u8> = vec![];

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
                                buf.extend(chunk);
                            }
                            Err(err) => return Err(err.into()),
                        }
                    } else {
                        break;
                    }
                }
                _ = tick.next() => {
                    if fire {
                        return Err(Error::Timeout);
                    } else {
                        fire = true;
                    }
                }
            }
        }

        // Decrypt ase128 encoded
        let de = if let (Some(key), Some(iv)) = (segment.key, segment.iv) {
            decrypt_aes128(&key[..], &iv[..], buf.as_ref())?
        } else {
            buf.to_vec()
        };

        loop {
            if self.shared_index.get() == index {
                if let Err(err) = self.sender.send((index, Bytes::from(de))).await {
                    return Err(Error::InnerError(format!(
                        "Error at `http::RequestTask`: Sender error: {:?}",
                        err
                    )));
                }
                self.shared_index.set(index + 1);
                return Ok(());
            } else {
                delay_for(Duration::from_millis(500)).await;
            }
        }
    }
}
