use std::{cell::RefCell, cmp::min, io::SeekFrom, rc::Rc, time::Duration};

use futures::{
    channel::mpsc::{channel, Receiver},
    select,
    stream::StreamExt,
};

use actix_rt::{spawn, time::interval, System};
use bytes::Bytes;

use crate::{
    app::Config,
    chunk::{make_range_chunks, RangePart, RangeStack},
    error::{AgetError, NetError, Result},
    printer::Printer,
    request::{get_content_length, get_redirect_uri, AgetRequestOptions, ContentLengthItem},
    store::{AgetFile, File, TaskInfo},
    task::RequestTask,
    util::QUIET,
};

pub struct CoreProcess {
    config: Config,
    options: AgetRequestOptions,
    range_stack: RangeStack,
    // the length of range_stack
    range_count: u64,
}

impl CoreProcess {
    pub fn new(config: Config) -> Result<CoreProcess> {
        let headers = &config
            .headers
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<&str>>();
        let data = config.data.as_ref().map(AsRef::as_ref);
        let options = AgetRequestOptions::new(&config.uri, &config.method, headers, data)?;

        Ok(CoreProcess {
            config,
            options,
            range_stack: Rc::new(RefCell::new(vec![])),
            range_count: 1,
        })
    }

    fn check_content_length(&self, content_length: u64) -> Result<()> {
        debug!("Check content length", content_length);
        let mut aget_file = AgetFile::new(&self.config.path)?;
        if aget_file.exists() {
            aget_file.open()?;
            if content_length != aget_file.content_length()? {
                debug!(
                    "!! the content length that response returned isn't equal of aget file",
                    format!("{} != {}", content_length, aget_file.content_length()?)
                );
                return Err(AgetError::ContentLengthIsNotConsistent.into());
            }
        }
        debug!("Check content length: equal");

        Ok(())
    }

    fn set_content_length(&self, content_length: u64) -> Result<()> {
        debug!("Set content length", content_length);
        let mut aget_file = AgetFile::new(&self.config.path)?;
        if !aget_file.exists() {
            aget_file.open()?;
            aget_file.write_content_length(content_length)?;
        } else {
            aget_file.open()?;
            aget_file.rewrite()?;
        }
        Ok(())
    }

    fn make_range_stack(&mut self) -> Result<()> {
        debug!("Make range stack");

        let mut range_stack: Vec<RangePart> = Vec::new();

        if self.options.is_concurrent() {
            let mut aget_file = AgetFile::new(&self.config.path)?;
            aget_file.open()?;
            let gaps = aget_file.gaps()?;

            let chunk_length = self.config.chunk_length;
            for gap in gaps.iter() {
                let mut list = make_range_chunks(gap, chunk_length);
                range_stack.append(&mut list);
            }
            range_stack.reverse();
        } else {
            range_stack.push(RangePart::new(0, 0));
        };

        self.range_count = range_stack.len() as u64;

        debug!("Range stack size", range_stack.len());
        self.range_stack = Rc::new(RefCell::new(range_stack));

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        // 1. Get redirected uri
        debug!("Redirect task");
        get_redirect_uri(&mut self.options).await?;

        // 2. Get content length
        debug!("ContentLength task");
        let cn_item = get_content_length(&mut self.options).await?;
        match cn_item {
            ContentLengthItem::RangeLength(content_length) => {
                self.check_content_length(content_length)?;
                self.set_content_length(content_length)?;
            }
            ContentLengthItem::DirectLength(content_length) => {
                self.set_content_length(content_length)?;
                self.options.no_concurrency();

                // Let connector to be always alive
                self.options.reset_connector(10, 60, 0);
            }
            ContentLengthItem::NoLength => {
                return Err(NetError::NoContentLength.into());
            }
        }
        self.make_range_stack()?;

        // 3. Spawn concurrent tasks
        let is_concurrent = self.options.is_concurrent();
        let (sender, receiver) =
            channel::<(RangePart, Bytes)>((self.config.concurrency + 1) as usize);
        let concurrency = if is_concurrent {
            self.config.concurrency
        } else {
            1
        };
        debug!("Spawn RequestTasks", concurrency);

        for i in 0..(min(self.config.concurrency, self.range_count)) {
            debug!("RequestTask ", i);
            let range_stack = self.range_stack.clone();
            let sender_ = sender.clone();
            let options = self.options.clone();
            let task = async {
                let is_concurrent = options.is_concurrent();
                let mut request_task = RequestTask::new(range_stack, sender_);
                let result = request_task.run(options).await;
                if let Err(err) = result {
                    print_err!("RequestTask fails", err);
                    if !is_concurrent {
                        // Exit process when the only one request task fails
                        System::current().stop();
                    }
                }
            };
            spawn(task);
        }

        // 4. Wait stream handler
        debug!("Start StreamHander");
        let stream_header = StreamHander::new(&self.config.path, !is_concurrent);
        if stream_header.is_err() {
            System::current().stop();
        }
        let stream_header = stream_header.unwrap();
        stream_header.run(receiver).await;

        debug!("CoreProcess done");
        Ok(())
    }
}

struct StreamHander {
    file: File,
    aget_file: AgetFile,
    task_info: TaskInfo,
    printer: Printer,
    no_record: bool,
}

impl StreamHander {
    fn new(path: &str, no_record: bool) -> Result<StreamHander, AgetError> {
        let task_info = TaskInfo::new(path)?;

        let mut file = File::new(path, false)?;
        file.open()?;
        let mut aget_file = AgetFile::new(path)?;
        aget_file.open()?;

        let printer = Printer::new();
        let mut handler = StreamHander {
            file,
            aget_file,
            task_info,
            printer,
            no_record,
        };
        handler.init_print()?;
        Ok(handler)
    }

    fn init_print(&mut self) -> Result<(), AgetError> {
        unsafe {
            if QUIET {
                return Ok(());
            }
        }

        if self.no_record {
            self.printer
                .print_msg("Server doesn't support range request.")?;
        }

        let file_name = &self.task_info.path;
        let content_length = self.task_info.content_length;
        self.printer.print_header(file_name)?;
        self.printer.print_length(content_length)?;
        self.print_process()?;
        Ok(())
    }

    fn print_process(&mut self) -> Result<(), AgetError> {
        unsafe {
            if QUIET {
                return Ok(());
            }
        }

        let total_length = self.task_info.content_length;
        let completed_length = self.task_info.completed_length();
        let (rate, eta) = self.task_info.rate_and_eta();
        self.printer
            .print_process(completed_length, total_length, rate, eta)?;
        Ok(())
    }

    fn record_range(&mut self, range_part: RangePart) -> Result<(), ()> {
        if self.no_record {
            return Ok(());
        }
        if let Err(err) = self.aget_file.write_interval(range_part) {
            print_err!("write interval to aget file fails", err);
            return Err(());
        }
        return Ok(());
    }

    fn teardown(&mut self) -> Result<(), AgetError> {
        self.aget_file.remove()?;
        Ok(())
    }

    pub async fn run(mut self, receiver: Receiver<(RangePart, Bytes)>) {
        debug!("StreamHander run");
        let mut tick = interval(Duration::from_secs(1)).fuse();
        let mut receiver = receiver.fuse();
        loop {
            select! {
                item = receiver.next() => {
                    if let Some((range, chunk)) = item {
                        let interval_length = range.length();

                        // write buf
                        if let Err(err) = self
                            .file
                            .write(&chunk[..], Some(SeekFrom::Start(range.start)))
                        {
                            print_err!("write chunk to file fails", err);
                        }

                        // write range_part
                        self.record_range(range);

                        // update `task_info`
                        self.task_info.add_completed(interval_length);
                    } else {
                        break;
                    }
                }
                _ = tick.next() => {
                    if let Err(err) = self.print_process() {
                        print_err!("print process fails", err);
                    }
                    self.task_info.clean_interval();
                    if self.task_info.remains() == 0 {
                        if let Err(err) = self.print_process() {
                            print_err!("print process fails", err);
                        }
                        if let Err(err) = self.teardown() {
                            print_err!("teardown stream handler fails", err);
                        }
                        break;
                    }
                }
            }
        }
    }
}
