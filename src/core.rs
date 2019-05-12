use std::cmp::min;
use std::io::SeekFrom;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::sync::mpsc::{channel, Receiver};
use futures::{try_ready, Async, Future, Poll, Stream};

use tokio::timer;

use actix::{spawn, Actor, Addr, System};
use actix_web::client::ClientConnector;

use bytes::Bytes;

use crate::app::Config;
use crate::chunk::{make_range_chunks, RangePart, RangeStack};
use crate::error::{AgetError, Error, NetError, Result};
use crate::printer::Printer;
use crate::request::{AgetRequestOptions, ContentLength, ContentLengthItem, Redirect};
use crate::store::{AgetFile, File, TaskInfo};
use crate::task::RequestTask;
use crate::util::QUIET;

enum InnerState {
    Redirect,
    ContentLength,
    Task,
    End,
}

pub struct CoreProcess {
    config: Config,
    state: InnerState,
    connector: Addr<ClientConnector>,
    options: AgetRequestOptions,
    range_stack: Option<RangeStack>,
    // the length of range_stack
    range_count: u64,
    redirect: Option<Redirect>,
    content_length: Option<ContentLength>,
}

impl CoreProcess {
    pub fn new(config: Config) -> Result<CoreProcess> {
        let connector = ClientConnector::default()
            .limit(0) // no limit simultaneous connections.
            .conn_keep_alive(Duration::from_secs(10))
            .conn_lifetime(Duration::from_secs(60 * 10))
            .start();
        let headers = &config
            .headers
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<&str>>();
        let data = config.data.as_ref().map(AsRef::as_ref);
        let options =
            AgetRequestOptions::new(&config.uri, &config.method, headers, data)?;

        Ok(CoreProcess {
            config,
            state: InnerState::Redirect,
            connector,
            options,
            range_stack: None,
            range_count: 1,
            redirect: None,
            content_length: None,
        })
    }

    fn set_connector(&mut self, connector: Addr<ClientConnector>) {
        self.connector = connector;
    }

    fn make_redirect(&mut self) -> &mut Self {
        debug!("Make Redirect task");
        let redirect = Redirect::new(self.options.clone(), self.connector.clone());
        self.redirect = Some(redirect);
        self
    }

    fn make_content_length(&mut self) -> &mut Self {
        debug!("Make ContentLength task");
        let content_length =
            ContentLength::new(self.options.clone(), self.connector.clone());
        self.content_length = Some(content_length);
        self
    }

    fn check_content_length(&self, content_length: u64) -> Result<()> {
        debug!("Check content length", content_length);
        let mut aget_file = AgetFile::new(&self.config.path)?;
        if aget_file.exists() {
            aget_file.open()?;
            if content_length != aget_file.content_length()? {
                debug!("!! the content length that response returned isn't equal of aget file",
                       format!("{} != {}", content_length, aget_file.content_length()?));
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
        self.range_stack = Some(Arc::new(Mutex::new(range_stack)));

        Ok(())
    }
}

impl Future for CoreProcess {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.state {
                InnerState::Redirect => {
                    if let Some(ref mut redirect) = self.redirect {
                        let new_uri = try_ready!(redirect.poll());
                        self.options.set_uri(new_uri.as_str());
                        self.state = InnerState::ContentLength;
                        self.redirect = None;
                    } else {
                        self.make_redirect();
                    }
                }
                InnerState::ContentLength => {
                    if let Some(ref mut content_length) = self.content_length {
                        let content_length = try_ready!(content_length.poll());

                        debug!("ContentLengthItem", content_length);

                        match content_length {
                            ContentLengthItem::RangeLength(content_length) => {
                                self.check_content_length(content_length)?;
                                self.set_content_length(content_length)?;
                            }
                            ContentLengthItem::DirectLength(content_length) => {
                                self.set_content_length(content_length)?;
                                self.options.no_concurrency();

                                // connector is always alive
                                let connector = ClientConnector::default()
                                    .limit(0) // no limit simultaneous connections.
                                    .conn_keep_alive(Duration::from_secs(10))
                                    .conn_lifetime(Duration::from_secs(0))
                                    .start();

                                self.set_connector(connector);
                            }
                            ContentLengthItem::NoLength => {
                                return Err(NetError::NoContentLength.into());
                            }
                        }

                        self.make_range_stack()?;
                        self.state = InnerState::Task;
                        self.content_length = None;
                    } else {
                        self.make_content_length();
                    }
                }
                InnerState::Task => {
                    if let Some(ref mut range_stack) = self.range_stack {
                        let is_concurrent = self.options.is_concurrent();
                        let (sender, receiver) = channel::<(RangePart, Bytes)>(
                            (self.config.concurrency + 1) as usize,
                        );

                        debug!("Spawn StreamHander");
                        let stream_header = StreamHander::new(
                            &self.config.path,
                            receiver,
                            !is_concurrent,
                        )?
                        .map(|_| {
                            System::current().stop();
                        });
                        spawn(stream_header);

                        let concurrency = if is_concurrent {
                            self.config.concurrency
                        } else {
                            1
                        };
                        debug!("Spawn RequestTasks", concurrency);
                        for _ in 0..(min(self.config.concurrency, self.range_count)) {
                            let task = RequestTask::new(
                                range_stack.clone(),
                                self.options.clone(),
                                self.connector.clone(),
                                sender.clone(),
                            )
                            .map_err(move |err| {
                                print_err!("RequestTask fails", err);
                                if !is_concurrent {
                                    // Exit process when the only one request task fails
                                    System::current().stop();
                                }
                            });
                            spawn(task);
                        }
                    } else {
                        unreachable!("Bug: No RangeStack");
                    }
                    self.state = InnerState::End;
                }
                InnerState::End => {
                    return Ok(Async::Ready(()));
                }
            }
        }
    }
}

enum Item {
    Value((RangePart, Bytes)),
    Tick,
}

struct StreamHander {
    stream: Box<dyn Stream<Item = Item, Error = Error>>,
    file: File,
    aget_file: AgetFile,
    task_info: TaskInfo,
    printer: Printer,
    no_record: bool,
}

impl StreamHander {
    fn new(
        path: &str,
        receiver: Receiver<(RangePart, Bytes)>,
        no_record: bool,
    ) -> Result<StreamHander, AgetError> {
        let task_info = TaskInfo::new(path)?;

        let mut file = File::new(path, false)?;
        file.open()?;
        let mut aget_file = AgetFile::new(path)?;
        aget_file.open()?;

        let tick = timer::Interval::new_interval(Duration::from_secs(2))
            .map_err(|err| AgetError::Bug(format!("tick error: {}", err)))
            .map(|_| Item::Tick);
        let stream = receiver
            .map_err(|_| AgetError::Bug("receiver error".to_owned()))
            .map(Item::Value)
            .select(tick)
            .from_err();

        let printer = Printer::new();
        let mut handler = StreamHander {
            stream: Box::new(stream),
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
}

impl Future for StreamHander {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.stream.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Ok(Async::Ready(item)) => {
                    match item {
                        Some(Item::Value((range_part, buf))) => {
                            let interval_length = range_part.length();

                            // write buf
                            if let Err(err) = self
                                .file
                                .write(&buf[..], Some(SeekFrom::Start(range_part.start)))
                            {
                                print_err!("write chunk to file fails", err);
                                return Err(());
                            }

                            // write range_part
                            self.record_range(range_part)?;

                            // update `task_info`
                            self.task_info.add_completed(interval_length);
                        }
                        Some(Item::Tick) => {
                            if let Err(err) = self.print_process() {
                                print_err!("print process fails", err);
                                return Err(());
                            }
                            self.task_info.clean_interval();
                            if self.task_info.remains() == 0 {
                                if let Err(err) = self.print_process() {
                                    print_err!("print process fails", err);
                                    return Err(());
                                }
                                if let Err(err) = self.teardown() {
                                    print_err!("teardown stream handler fails", err);
                                    return Err(());
                                }
                                return Ok(Async::Ready(()));
                            }
                        }
                        // never reach here !!!
                        None => {
                            return Ok(Async::Ready(()));
                        }
                    }
                }
                Err(err) => {
                    print_err!("stream error", err);
                    return Err(());
                }
            }
        }
    }
}
