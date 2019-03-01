use std::cmp::max;
use std::fs::{remove_file, File as StdFile, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::Instant;

use crate::chunk::RangePart;
use crate::common::AGET_EXT;
use crate::error::{AgetError, Result};

pub struct TaskInfo {
    pub path: String,

    /// The length of the file
    pub content_length: u64,

    /// The length stored to the file
    completed_length: u64,

    /// The stored length at an interval of one tick
    interval_length: u64,

    /// The interval of one tick
    tick_interval: Instant,
}

impl TaskInfo {
    pub fn new(path: &str) -> Result<TaskInfo, AgetError> {
        let mut aget_file = AgetFile::new(path)?;
        aget_file.open()?;

        let path = path.to_string();
        let content_length = aget_file.content_length()?;
        let completed_length = aget_file.completed_length()?;

        Ok(TaskInfo {
            path,
            content_length,
            completed_length,
            interval_length: 0,
            tick_interval: Instant::now(),
        })
    }

    pub fn completed_length(&self) -> u64 {
        self.completed_length
    }

    pub fn remains(&self) -> u64 {
        self.content_length - self.completed_length
    }

    pub fn rate_and_eta(&self) -> (f64, u64) {
        let interval = self.tick_interval.elapsed().as_secs();
        let rate = self.interval_length as f64 / interval as f64;
        let remains = self.remains();
        // rate > 1.0 for overflow
        let eta = if remains > 0 && rate > 1.0 {
            let eta = (remains as f64 / rate) as u64;
            // eta is large than 99 days, return 0
            if eta > 99 * 24 * 60 * 60 {
                0
            } else {
                eta
            }
        } else {
            0
        };
        (rate, eta)
    }

    pub fn add_completed(&mut self, interval_length: u64) {
        self.completed_length += interval_length;
        self.interval_length += interval_length;
    }

    pub fn clean_interval(&mut self) {
        self.interval_length = 0;
        self.tick_interval = Instant::now();
    }
}

pub struct File {
    path: PathBuf,
    file: Option<StdFile>,
    readable: bool,
}

impl File {
    pub fn new(p: &str, readable: bool) -> Result<File, AgetError> {
        let path = PathBuf::from(p);
        if path.is_dir() {
            return Err(AgetError::InvalidPath(p.to_string()));
        }

        Ok(File {
            path,
            file: None,
            readable,
        })
    }

    pub fn open(&mut self) -> Result<&mut Self, AgetError> {
        let file = OpenOptions::new()
            .read(self.readable)
            .write(true)
            .truncate(false)
            .create(true)
            .open(self.path.as_path())?;
        self.file = Some(file);
        Ok(self)
    }

    pub fn exists(&self) -> bool {
        self.path.as_path().exists()
    }

    pub fn file_name(&self) -> Result<String, AgetError> {
        if let Some(file_name) = self.path.as_path().file_name() {
            Ok(file_name.to_str().unwrap().to_string())
        } else {
            Err(AgetError::NoFilename)
        }
    }

    pub fn file(&mut self) -> Result<&mut StdFile, AgetError> {
        if let Some(ref mut file) = self.file {
            Ok(file)
        } else {
            Err(AgetError::Bug(
                "`store::File::file` must be opened".to_string(),
            ))
        }
    }

    pub fn write(&mut self, buf: &[u8], seek: Option<SeekFrom>) -> Result<usize, AgetError> {
        if let Some(seek) = seek {
            self.seek(seek)?;
        }
        let s = self.file()?.write(buf)?;
        Ok(s)
    }

    pub fn read(&mut self, buf: &mut [u8], seek: Option<SeekFrom>) -> Result<usize, AgetError> {
        if let Some(seek) = seek {
            self.seek(seek)?;
        }
        let s = self.file()?.read(buf)?;
        Ok(s)
    }

    pub fn seek(&mut self, seek: SeekFrom) -> Result<u64, AgetError> {
        let s = self.file()?.seek(seek)?;
        Ok(s)
    }

    pub fn set_len(&mut self, size: u64) -> Result<(), AgetError> {
        self.file()?.set_len(size)?;
        Ok(())
    }

    pub fn remove(&self) -> Result<(), AgetError> {
        remove_file(self.path.as_path())?;
        Ok(())
    }
}

/// Aget Information Store File
///
/// The file stores two kinds of information which of the downloading file.
/// 1. Content Length
///   The downloading file's content length.
///   If the number `request::ContentLength` returned is not equal to this content length,
///   the process will be terminated.
/// 2. Close Intervals of Downloaded Pieces
///   These intervals are pieces `(u64, u64)` stored as big-endian.
///   First item is the begin of header `Range`.
///   Second item is the end of header `Range`.
pub struct AgetFile {
    inner: File,
}

impl AgetFile {
    pub fn new(path: &str) -> Result<AgetFile, AgetError> {
        let mut path = path.to_string();
        path.push_str(AGET_EXT);

        let file = File::new(&path, true)?;
        Ok(AgetFile { inner: file })
    }

    pub fn open(&mut self) -> Result<&mut Self, AgetError> {
        self.inner.open()?;
        Ok(self)
    }

    pub fn file_name(&self) -> Result<String, AgetError> {
        self.inner.file_name()
    }

    pub fn exists(&self) -> bool {
        self.inner.exists()
    }

    pub fn remove(&self) -> Result<(), AgetError> {
        self.inner.remove()
    }

    /// Get downloading file's content length stored in the aget file
    pub fn content_length(&mut self) -> Result<u64, AgetError> {
        let mut buf: [u8; 8] = [0; 8];
        self.inner.read(&mut buf, Some(SeekFrom::Start(0)))?;
        let content_length = u8x8_to_u64(&buf);
        Ok(content_length)
    }

    pub fn completed_length(&mut self) -> Result<u64, AgetError> {
        let completed_intervals = self.completed_intervals()?;
        if completed_intervals.is_empty() {
            Ok(0)
        } else {
            Ok(completed_intervals.iter().map(RangePart::length).sum())
        }
    }

    pub fn completed_intervals(&mut self) -> Result<Vec<RangePart>, AgetError> {
        let mut intervals: Vec<(u64, u64)> = Vec::new();

        let mut buf: [u8; 16] = [0; 16];
        self.inner.seek(SeekFrom::Start(8))?;
        loop {
            let s = self.inner.read(&mut buf, None)?;
            if s != 16 {
                break;
            }

            let mut raw = [0; 8];
            raw.clone_from_slice(&buf[..8]);
            let start = u8x8_to_u64(&raw);
            raw.clone_from_slice(&buf[8..]);
            let end = u8x8_to_u64(&raw);

            assert!(
                start <= end,
                format!(
                    "Bug: `start > end` in an interval of aget file. : {} > {}",
                    start, end
                )
            );

            intervals.push((start, end));
        }

        intervals.sort();

        // merge intervals
        let mut merge_intervals: Vec<(u64, u64)> = Vec::new();
        if !intervals.is_empty() {
            merge_intervals.push(intervals[0]);
        }
        for (start, end) in intervals.iter() {
            let (pre_start, pre_end) = merge_intervals.last().unwrap().clone();

            // case 1
            // ----------
            //                -----------
            if pre_end + 1 < *start {
                merge_intervals.push((*start, *end));
            // case 2
            // -----------------
            //                  ----------
            //             --------
            //     ------
            } else {
                let n_start = pre_start;
                let n_end = max(pre_end, *end);
                merge_intervals.pop();
                merge_intervals.push((n_start, n_end));
            }
        }

        Ok(merge_intervals
            .iter()
            .map(|(start, end)| RangePart::new(*start, *end))
            .collect::<Vec<RangePart>>())
    }

    /// Get gaps of undownload pieces
    pub fn gaps(&mut self) -> Result<Vec<RangePart>, AgetError> {
        let mut completed_intervals = self.completed_intervals()?;
        let content_length = self.content_length()?;
        completed_intervals.push(RangePart::new(content_length, content_length));

        // find gaps
        let mut gaps: Vec<RangePart> = Vec::new();
        // find first chunk
        let RangePart { start, .. } = completed_intervals[0];
        if start > 0 {
            gaps.push(RangePart::new(0, start - 1));
        }

        for (index, RangePart { end, .. }) in completed_intervals.iter().enumerate() {
            if let Some(RangePart {
                start: next_start, ..
            }) = completed_intervals.get(index + 1)
            {
                if end + 1 < *next_start {
                    gaps.push(RangePart::new(end + 1, next_start - 1));
                }
            }
        }

        Ok(gaps)
    }

    pub fn write_content_length(&mut self, content_length: u64) -> Result<(), AgetError> {
        let buf = u64_to_u8x8(content_length);
        self.inner.write(&buf, Some(SeekFrom::Start(0)))?;
        Ok(())
    }

    pub fn write_interval(&mut self, interval: RangePart) -> Result<(), AgetError> {
        let start = u64_to_u8x8(interval.start);
        let end = u64_to_u8x8(interval.end);
        let buf = [start, end].concat();
        self.inner.write(&buf, Some(SeekFrom::End(0)))?;
        Ok(())
    }

    // Merge completed intervals and rewrite the aget file
    pub fn rewrite(&mut self) -> Result<(), AgetError> {
        let content_length = self.content_length()?;
        let completed_intervals = self.completed_intervals()?;

        let mut buf: Vec<u8> = Vec::new();
        buf.extend(&u64_to_u8x8(content_length));
        for interval in completed_intervals.iter() {
            buf.extend(&u64_to_u8x8(interval.start));
            buf.extend(&u64_to_u8x8(interval.end));
        }

        self.inner.set_len(0)?;
        self.inner.write(buf.as_slice(), Some(SeekFrom::Start(0)))?;

        Ok(())
    }
}

/// Create an integer value from its representation as a byte array in big endian.
pub fn u8x8_to_u64(u8x8: &[u8; 8]) -> u64 {
    u64::from_be_bytes(*u8x8)
}

/// Return the memory representation of this integer as a byte array in big-endian (network) byte order.
pub fn u64_to_u8x8(u: u64) -> [u8; 8] {
    u.to_be_bytes()
}
