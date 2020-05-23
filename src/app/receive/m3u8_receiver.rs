use std::{io::SeekFrom, path::Path, time::Duration};

use async_std::stream;
use futures::{channel::mpsc::Receiver, select, stream::StreamExt};

use crate::{
    app::{
        show::m3u8_show::M3u8Shower,
        stats::list_stats::{ListStats, LISTSTATS_FILE_SUFFIX},
        status::rate_status::RateStatus,
    },
    common::{bytes::bytes_type::Bytes, errors::Result, file::File},
};

pub struct M3u8Receiver {
    output: File,
    liststats: ListStats,
    ratestatus: RateStatus,
    shower: M3u8Shower,
    // Total number of the `SharedM3u8SegmentList`
    total: u64,
    completed: u64,
}

impl M3u8Receiver {
    pub fn new<P: AsRef<Path>>(output: P) -> Result<M3u8Receiver> {
        let mut outputfile = File::new(&output, true)?;
        outputfile.open()?;

        let mut liststats =
            ListStats::new(&*(output.as_ref().to_string_lossy() + LISTSTATS_FILE_SUFFIX))?;
        liststats.open()?;
        let total = liststats.total()?;
        let completed = liststats.index()?;

        Ok(M3u8Receiver {
            output: outputfile,
            liststats,
            ratestatus: RateStatus::new(),
            shower: M3u8Shower::new(),
            total,
            completed,
        })
    }

    fn show_infos(&mut self) -> Result<()> {
        let file_name = &self.output.file_name().unwrap_or("[No Name]");
        let total = self.total;
        self.shower.print_file(file_name)?;
        self.shower.print_total(total)?;
        self.show_status()?;
        Ok(())
    }

    fn show_status(&mut self) -> Result<()> {
        let total = self.total;
        let completed = self.completed;
        let rate = self.ratestatus.rate();

        self.shower.print_status(completed, total, rate)?;
        self.ratestatus.clean();
        Ok(())
    }

    pub async fn start(&mut self, receiver: Receiver<(u64, Bytes)>) -> Result<()> {
        self.show_infos()?;

        let mut tick = stream::interval(Duration::from_secs(2)).fuse();
        let mut receiver = receiver.fuse();
        loop {
            select! {
                item = receiver.next() => {
                    if let Some((index, chunk)) = item {
                        self.output.write(&chunk[..], Some(SeekFrom::End(0)))?;
                        self.liststats.write_index(index + 1)?;
                        self.ratestatus.add(chunk.len() as u64);
                        self.completed = index + 1;
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
