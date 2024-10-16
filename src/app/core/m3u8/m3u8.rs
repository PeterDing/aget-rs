use std::{
    cell::Cell,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{
    channel::mpsc::{channel, Sender},
    pin_mut, select, SinkExt, StreamExt,
};

use crate::{
    app::{
        core::m3u8::common::{get_m3u8, M3u8Segment, SharedM3u8SegmentList},
        receive::m3u8_receiver::M3u8Receiver,
        record::{bytearray_recorder::ByteArrayRecorder, common::RECORDER_FILE_SUFFIX},
    },
    common::{
        bytes::bytes_type::Bytes,
        crypto::decrypt_aes128,
        errors::{Error, Result},
        net::{
            net::{build_http_client, request},
            HttpClient, Method, Url,
        },
        time::interval_stream,
    },
    features::{args::Args, running::Runnable, stack::StackLike},
};

/// M3u8 task handler
pub struct M3u8Handler<'a> {
    output: PathBuf,
    method: Method,
    url: Url,
    data: Option<&'a str>,
    concurrency: u64,
    timeout: Duration,
    client: HttpClient,
}

impl<'a> M3u8Handler<'a> {
    pub fn new(args: &impl Args) -> Result<M3u8Handler> {
        let headers = args.headers();
        let timeout = args.timeout();
        let dns_timeout = args.dns_timeout();
        let keep_alive = args.keep_alive();
        let skip_verify_tls_cert = args.skip_verify_tls_cert();
        let proxy = args.proxy();

        let client = build_http_client(&headers, timeout, dns_timeout, keep_alive, skip_verify_tls_cert, proxy)?;

        tracing::debug!("M3u8Handler::new");

        Ok(M3u8Handler {
            output: args.output(),
            method: args.method(),
            url: args.url(),
            data: args.data(),
            concurrency: args.concurrency(),
            timeout,
            client,
        })
    }

    async fn start(self) -> Result<()> {
        tracing::debug!("M3u8Handler::start");

        // 0. Check whether task is completed
        tracing::debug!("M3u8Handler: check whether task is completed");
        let mut bytearrayrecorder = ByteArrayRecorder::new(&*(self.output.to_string_lossy() + RECORDER_FILE_SUFFIX))?;
        if self.output.exists() && !bytearrayrecorder.exists() {
            return Ok(());
        }

        // 1. Get m3u8 info
        tracing::debug!("M3u8Handler: get m3u8");
        let mut ls = get_m3u8(
            &self.client,
            self.method.clone(),
            self.url.clone(),
            self.data.map(|v| v.to_string()),
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
        tracing::debug!("M3u8Handler: segments: {}", stack.len());

        // 3. Create channel
        let (sender, receiver) = channel::<(u64, Bytes)>(self.concurrency as usize + 10);
        let runtime_error: Arc<Mutex<Option<Error>>> = Arc::new(Mutex::new(None));

        // 4. Spawn request task
        let concurrency = std::cmp::min(stack.len() as u64, self.concurrency);
        for i in 1..concurrency + 1 {
            let mut task = RequestTask::new(
                self.client.clone(),
                stack.clone(),
                sender.clone(),
                i,
                sharedindex.clone(),
                self.timeout,
            );
            let runtime_error_clone = runtime_error.clone();
            actix_rt::spawn(async move {
                if let Err(err) = task.start().await {
                    if runtime_error_clone.lock().unwrap().is_none() {
                        *runtime_error_clone.lock().unwrap() = Some(err);
                    }
                }
            });
        }
        drop(sender); // Remove the reference and let `Task` to handle it

        // 5. Create receiver
        tracing::debug!("M3u8Handler: create receiver");
        let mut m3u8receiver = M3u8Receiver::new(&self.output)?;
        m3u8receiver.start(receiver).await?;

        if let Some(err) = runtime_error.lock().unwrap().take() {
            return Err(err);
        }

        // 6. Fixup output file

        // 7. Task succeeds. Remove `ByteArrayRecorder` file
        bytearrayrecorder.remove().unwrap_or(()); // Missing error
        Ok(())
    }
}

impl<'a> Runnable for M3u8Handler<'a> {
    fn run(self) -> Result<()> {
        let sys = actix_rt::System::new();
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

    async fn start(&mut self) -> Result<()> {
        tracing::debug!("Fire RequestTask: {}", self.id);
        while let Some(segment) = self.stack.pop() {
            loop {
                match self.req(segment.clone()).await {
                    // Exit whole process when `Error::InnerError` is returned
                    Err(Error::InnerError(msg)) => {
                        tracing::error!("RequestTask {}: InnerError: {}", self.id, msg);
                        actix_rt::System::current().stop();
                    }
                    Err(err @ Error::Timeout) => {
                        tracing::debug!("RequestTask timeout: {:?}", err); // Missing Timeout at runtime
                    }
                    Err(err) => {
                        tracing::debug!("RequestTask {}: error: {:?}", self.id, err);
                        return Err(err);
                    }
                    _ => break,
                }
            }
        }
        Ok(())
    }

    async fn req(&mut self, segment: M3u8Segment) -> Result<()> {
        let resp = request(
            &self.client,
            segment.method.clone(),
            segment.url.clone(),
            segment.data.clone(),
            None,
        )
        .await?;

        let index = segment.index;

        // !!! resp.body().await can be overflow
        let mut buf: Vec<u8> = vec![];

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
                actix_rt::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}
