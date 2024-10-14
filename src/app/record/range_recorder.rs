use std::{cmp::max, io::SeekFrom, path::Path};

use crate::common::{
    bytes::bytes::{u64_to_u8x8, u8x8_to_u64},
    errors::Result,
    file::File,
    range::{RangeList, RangePair},
};

/// Range recorder
///
/// This struct records pairs which are `common::range::RangePair`.
/// All information is stored at a local file.
///
/// [total 8bit][ [begin1 8bit,end1 8bit] [begin2 8bit,end2 8bit] ... ]
/// `total` position is not sum_i{end_i - begin_i + 1}. It is given by
/// user, presenting as the real total number.
pub struct RangeRecorder {
    inner: File,
}

impl RangeRecorder {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<RangeRecorder> {
        let inner = File::new(path, true)?;
        Ok(RangeRecorder { inner })
    }

    pub fn open(&mut self) -> Result<&mut Self> {
        self.inner.open()?;
        Ok(self)
    }

    pub fn file_name(&self) -> Option<&str> {
        self.inner.file_name()
    }

    pub fn exists(&self) -> bool {
        self.inner.exists()
    }

    /// Delete the inner file
    pub fn remove(&self) -> Result<()> {
        self.inner.remove()
    }

    /// Get downloading file's content length stored in the aget file
    pub fn total(&mut self) -> Result<u64> {
        let mut buf: [u8; 8] = [0; 8];
        self.inner.read(&mut buf, Some(SeekFrom::Start(0)))?;
        let cl = u8x8_to_u64(&buf);
        Ok(cl)
    }

    /// Count the length of total pairs
    pub fn count(&mut self) -> Result<u64> {
        let pairs = self.pairs()?;
        if pairs.is_empty() {
            Ok(0)
        } else {
            Ok(pairs.iter().map(RangePair::length).sum())
        }
    }

    /// Recorded pairs
    pub fn pairs(&mut self) -> Result<RangeList> {
        let mut pairs: Vec<(u64, u64)> = Vec::new();

        let mut buf: [u8; 16] = [0; 16];
        self.inner.seek(SeekFrom::Start(8))?;
        loop {
            let s = self.inner.read(&mut buf, None)?;
            if s != 16 {
                break;
            }

            let mut raw = [0; 8];
            raw.clone_from_slice(&buf[..8]);
            let begin = u8x8_to_u64(&raw);
            raw.clone_from_slice(&buf[8..]);
            let end = u8x8_to_u64(&raw);

            assert!(
                begin <= end,
                "Bug: `begin > end` in an pair of {}. : {} > {}",
                self.file_name().unwrap_or(""),
                begin,
                end
            );

            pairs.push((begin, end));
        }

        pairs.sort_unstable();

        // merge pairs
        let mut merged_pairs: Vec<(u64, u64)> = Vec::new();
        if !pairs.is_empty() {
            merged_pairs.push(pairs[0]);
        }
        for (begin, end) in pairs.iter() {
            let (pre_start, pre_end) = *merged_pairs.last().unwrap();

            // case 1
            // ----------
            //                -----------
            if pre_end + 1 < *begin {
                merged_pairs.push((*begin, *end));
            // case 2
            // -----------------
            //                  ----------
            //             --------
            //     ------
            } else {
                let n_start = pre_start;
                let n_end = max(pre_end, *end);
                merged_pairs.pop();
                merged_pairs.push((n_start, n_end));
            }
        }

        Ok(merged_pairs
            .iter()
            .map(|(begin, end)| RangePair::new(*begin, *end))
            .collect::<RangeList>())
    }

    /// Get gaps between all pairs
    /// Each of gap is a closed interval
    pub fn gaps(&mut self) -> Result<RangeList> {
        let mut pairs = self.pairs()?;
        let total = self.total()?;
        pairs.push(RangePair::new(total, total));

        // find gaps
        let mut gaps: RangeList = Vec::new();
        // find first chunk
        let RangePair { begin, .. } = pairs[0];
        if begin > 0 {
            gaps.push(RangePair::new(0, begin - 1));
        }

        for (index, RangePair { end, .. }) in pairs.iter().enumerate() {
            if let Some(RangePair { begin: next_start, .. }) = pairs.get(index + 1) {
                if end + 1 < *next_start {
                    gaps.push(RangePair::new(end + 1, next_start - 1));
                }
            }
        }

        Ok(gaps)
    }

    pub fn write_total(&mut self, total: u64) -> Result<()> {
        let buf = u64_to_u8x8(total);
        self.inner.write(&buf, Some(SeekFrom::Start(0)))?;
        Ok(())
    }

    pub fn write_pair(&mut self, pair: RangePair) -> Result<()> {
        let begin = u64_to_u8x8(pair.begin);
        let end = u64_to_u8x8(pair.end);
        let buf = [begin, end].concat();
        self.inner.write(&buf, Some(SeekFrom::End(0)))?;
        Ok(())
    }

    // Merge completed pairs and rewrite the aget file
    pub fn rewrite(&mut self) -> Result<()> {
        let total = self.total()?;
        let pairs = self.pairs()?;

        let mut buf: Vec<u8> = Vec::new();
        buf.extend(&u64_to_u8x8(total));
        for pair in pairs.iter() {
            buf.extend(&u64_to_u8x8(pair.begin));
            buf.extend(&u64_to_u8x8(pair.end));
        }

        // Clean all content of the file and set its length to zero
        self.inner.set_len(0)?;

        // Write new data
        self.inner.write(buf.as_slice(), Some(SeekFrom::Start(0)))?;

        Ok(())
    }
}
