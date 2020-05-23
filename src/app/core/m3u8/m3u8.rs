use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use async_std::{io::ReadExt, process::exit, task as std_task};

use futures::{
    channel::mpsc::{channel, Sender},
    SinkExt,
};

use crate::{
    app::{
        core::m3u8::common::{get_m3u8, M3u8Segment, SharedM3u8SegmentList},
        receive::m3u8_receiver::M3u8Receiver,
        stats::list_stats::{ListStats, LISTSTATS_FILE_SUFFIX},
    },
    common::{
        bytes::bytes_type::{Buf, Bytes},
        crypto::decrypt_aes128,
        errors::{Error, Result},
        net::{
            net::{build_http_client, request},
            net_type::{HttpClient, Method, Uri},
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
    timeout: u64,
    concurrency: u64,
    proxy: Option<String>,
    client: Arc<HttpClient>,
}

impl M3u8Handler {
    pub fn new(args: &impl Args) -> Result<M3u8Handler> {
        let headers = args.headers();
        let timeout = args.timeout();
        let proxy = args.proxy();

        let hds: Vec<(&str, &str)> = headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let client = build_http_client(hds.as_ref(), timeout, proxy.as_deref())?;

        debug!("M3u8Handler::new");

        Ok(M3u8Handler {
            output: args.output(),
            method: args.method(),
            uri: args.uri(),
            headers,
            data: args.data().map(|ref mut d| d.to_bytes()),
            timeout,
            concurrency: args.concurrency(),
            proxy,
            client: Arc::new(client),
        })
    }

    async fn start(&mut self) -> Result<()> {
        debug!("M3u8Handler::start");

        // 0. Check whether task is completed
        debug!("M3u8Handler: check whether task is completed");
        let mut liststats =
            ListStats::new(&*(self.output.to_string_lossy() + LISTSTATS_FILE_SUFFIX))?;
        if self.output.exists() && !liststats.exists() {
            return Ok(());
        }

        // 1. Redirect
        debug!("M3u8Handler: get m3u8");
        let mut ls = get_m3u8(
            &self.client,
            self.method.clone(),
            self.uri.clone(),
            self.data.clone(),
        )
        .await?;
        ls.reverse();

        if liststats.exists() {
            liststats.open()?;
            let total = liststats.total()?;
            if total != ls.len() as u64 {
                return Err(Error::PartsAreNotConsistent);
            } else {
                let index = liststats.index()?;
                ls.truncate((total - index) as usize);
            }
        } else {
            liststats.open()?;
            liststats.write_total(ls.len() as u64)?;
        }

        let index = ls.last().unwrap().index;
        let sharedindex = Arc::new(AtomicU64::new(index));
        let stack = SharedM3u8SegmentList::new(ls);
        debug!("M3u8Handler: segments", stack.len());

        // 4. Create channel
        let (sender, receiver) = channel::<(u64, Bytes)>(self.concurrency as usize + 10);

        let concurrency = std::cmp::min(stack.len() as u64, self.concurrency);
        for i in 1..concurrency + 1 {
            let mut task = RequestTask::new(
                self.client.clone(),
                stack.clone(),
                sender.clone(),
                i,
                sharedindex.clone(),
            );
            std_task::spawn(async move {
                task.start().await;
            });
        }
        drop(sender); // Remove the reference and let `Task` to handle it

        // 6. Create receiver
        debug!("M3u8Handler: create receiver");
        let mut m3u8receiver = M3u8Receiver::new(&self.output)?;
        m3u8receiver.start(receiver).await?;

        // 7. Task succeeds. Remove liststats file
        liststats.remove().unwrap_or(()); // Missing error
        Ok(())
    }
}

impl Runnable for M3u8Handler {
    fn run(&mut self) -> Result<()> {
        std_task::block_on(self.start())
    }
}

/// Request the resource with a range header which is in the `SharedRangList`
struct RequestTask {
    client: Arc<HttpClient>,
    stack: SharedM3u8SegmentList,
    sender: Sender<(u64, Bytes)>,
    id: u64,
    shared_index: Arc<AtomicU64>,
}

impl RequestTask {
    fn new(
        client: Arc<HttpClient>,
        stack: SharedM3u8SegmentList,
        sender: Sender<(u64, Bytes)>,
        id: u64,
        sharedindex: Arc<AtomicU64>,
    ) -> RequestTask {
        RequestTask {
            client,
            stack,
            sender,
            id,
            shared_index: sharedindex,
        }
    }

    async fn start(&mut self) {
        while let Some(segment) = self.stack.pop() {
            loop {
                match self.req(segment.clone()).await {
                    // Exit whole process when `Error::InnerError` is returned
                    Err(Error::InnerError(msg)) => {
                        print_err!(format!("RequestTask {}: InnerError", self.id), msg);
                        exit(1);
                    }
                    Err(err) => {
                        print_err!(format!("RequestTask {}: error", self.id), err);
                        std_task::sleep(Duration::from_secs(1)).await;
                    }
                    _ => break,
                }
            }
        }
    }

    async fn req(&mut self, segment: M3u8Segment) -> Result<()> {
        let resp = request(
            &*self.client,
            segment.method.clone(),
            segment.uri.clone(),
            segment.data.clone(),
            None,
        )
        .await?;

        let index = segment.index;
        let mut buf: Vec<u8> = vec![];
        let mut reader = resp.into_body();
        reader.read_to_end(&mut buf).await?;
        if let (Some(key), Some(iv)) = (segment.key, segment.iv) {
            buf = decrypt_aes128(&key[..], &iv[..], &buf[..])?;
        }

        loop {
            if self.shared_index.load(Ordering::SeqCst) == index {
                if let Err(err) = self.sender.send((index, Bytes::from(buf))).await {
                    return Err(Error::InnerError(format!(
                        "Error at `http::RequestTask`: Sender error: {:?}",
                        err
                    )));
                }
                self.shared_index.store(index + 1, Ordering::SeqCst);
                return Ok(());
            } else {
                std_task::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}