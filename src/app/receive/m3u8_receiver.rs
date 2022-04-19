use std::{io::SeekFrom, path::Path, time::Duration};

use futures::{channel::mpsc::Receiver, pin_mut, select, StreamExt};

use crate::{
    app::{
        record::{bytearray_recorder::ByteArrayRecorder, common::RECORDER_FILE_SUFFIX},
        show::m3u8_show::M3u8Shower,
        status::rate_status::RateStatus,
    },
    common::{bytes::bytes_type::Bytes, errors::Result, file::File, time::interval_stream},
};

pub struct M3u8Receiver {
    output: File,
    bytearrayrecorder: ByteArrayRecorder,
    ratestatus: RateStatus,
    shower: M3u8Shower,
    total: u64,
    completed: u64,
    seek: u64,
}

impl M3u8Receiver {
    pub fn new<P: AsRef<Path>>(output: P) -> Result<M3u8Receiver> {
        let mut outputfile = File::new(&output, true)?;
        outputfile.open()?;

        // Record 3 variables in a `ByteArrayRecorder`:
        // [0-index, total segment number][1-index, completed segment number][2-index, seek offset]
        let mut bytearrayrecorder =
            ByteArrayRecorder::new(&*(output.as_ref().to_string_lossy() + RECORDER_FILE_SUFFIX))?;
        bytearrayrecorder.open()?;
        let total = bytearrayrecorder.index(0)?;
        let completed = bytearrayrecorder.index(1)?;
        let seek = bytearrayrecorder.index(2)?;

        Ok(M3u8Receiver {
            output: outputfile,
            bytearrayrecorder,
            ratestatus: RateStatus::new(),
            shower: M3u8Shower::new(),
            total,
            completed,
            seek,
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
        let length = self.seek;

        self.shower.print_status(completed, total, length, rate)?;
        self.ratestatus.clean();
        Ok(())
    }

    pub async fn start(&mut self, receiver: Receiver<(u64, Bytes)>) -> Result<()> {
        self.show_infos()?;

        let receiver = receiver.fuse();
        let tick = interval_stream(Duration::from_secs(2)).fuse();

        pin_mut!(receiver, tick);

        loop {
            select! {
                item = receiver.next() => {
                    if let Some((index, chunk)) = item {
                        let len = chunk.len() as u64;

                        // Write chunk to file
                        self.output.write(&chunk[..], Some(SeekFrom::Start(self.seek)))?;

                        // Record info
                        self.bytearrayrecorder.write(1, index + 1)?; // Write completed
                        self.bytearrayrecorder.write(2, self.seek + len)?; // Write seek offset
                        self.completed = index + 1;
                        self.seek += len ;

                        // Update rate
                        self.ratestatus.add(len);
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
