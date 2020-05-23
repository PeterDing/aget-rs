use std::{io::SeekFrom, path::Path, time::Duration};

use async_std::stream;
use futures::{channel::mpsc::Receiver, select, stream::StreamExt};

use crate::{
    app::{
        record::{common::RECORDER_FILE_SUFFIX, range_recorder::RangeRecorder},
        show::http_show::HttpShower,
        status::rate_status::RateStatus,
    },
    common::{bytes::bytes_type::Bytes, errors::Result, file::File, range::RangePair},
};

pub struct HttpReceiver {
    output: File,
    rangerecorder: Option<RangeRecorder>,
    ratestatus: RateStatus,
    shower: HttpShower,
    // Total content length of the uri
    total: u64,
}

impl HttpReceiver {
    pub fn new<P: AsRef<Path>>(output: P, direct: bool) -> Result<HttpReceiver> {
        let mut outputfile = File::new(&output, true)?;
        outputfile.open()?;

        let (rangerecorder, total, completed) = if direct {
            (None, 0, 0)
        } else {
            let mut rangerecorder =
                RangeRecorder::new(&*(output.as_ref().to_string_lossy() + RECORDER_FILE_SUFFIX))?;
            rangerecorder.open()?;
            let total = rangerecorder.total()?;
            let completed = rangerecorder.count()?;
            (Some(rangerecorder), total, completed)
        };

        let mut ratestatus = RateStatus::new();
        ratestatus.set_total(completed);

        Ok(HttpReceiver {
            output: outputfile,
            rangerecorder,
            ratestatus,
            shower: HttpShower::new(),
            // receiver,
            total,
        })
    }

    fn show_infos(&mut self) -> Result<()> {
        if self.rangerecorder.is_none() {
            self.shower
                .print_msg("Server doesn't support range request.")?;
        }

        let file_name = &self.output.file_name().unwrap_or("[No Name]");
        let total = self.total;
        self.shower.print_file(file_name)?;
        self.shower.print_total(total)?;
        // self.shower.print_concurrency(concurrency)?;
        self.show_status()?;
        Ok(())
    }

    fn show_status(&mut self) -> Result<()> {
        let total = self.total;
        let completed = self.ratestatus.total();
        let rate = self.ratestatus.rate();

        let eta = if self.rangerecorder.is_some() {
            let remains = total - completed;
            // rate > 1.0 for overflow
            if remains > 0 && rate > 1.0 {
                let eta = (remains as f64 / rate) as u64;
                // eta is large than 99 days, return 0
                if eta > 99 * 24 * 60 * 60 {
                    0
                } else {
                    eta
                }
            } else {
                0
            }
        } else {
            0
        };

        self.shower.print_status(completed, total, rate, eta)?;
        self.ratestatus.clean();
        Ok(())
    }

    fn record_pair(&mut self, pair: RangePair) -> Result<()> {
        if let Some(ref mut rangerecorder) = self.rangerecorder {
            rangerecorder.write_pair(pair)?;
        }
        Ok(())
    }
    pub async fn start(&mut self, receiver: Receiver<(RangePair, Bytes)>) -> Result<()> {
        self.show_infos()?;

        let mut tick = stream::interval(Duration::from_secs(2)).fuse();
        let mut receiver = receiver.fuse();
        loop {
            select! {
                item = receiver.next() => {
                    if let Some((pair, chunk)) = item {
                        self.output.write(&chunk[..], Some(SeekFrom::Start(pair.begin)))?;
                        self.record_pair(pair)?;
                        self.ratestatus.add(pair.length());
                    } else {
                        break;
                    }
                },
                _ = tick.next() => {
                    self.show_status()?;
                },
            }
        }
        self.show_status()?;
        Ok(())
    }
}
