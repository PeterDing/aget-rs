use std::{io::SeekFrom, path::Path};

use crate::common::{
    bytes::bytes::{u64_to_u8x8, u8x8_to_u64},
    errors::Result,
    file::File,
};

pub const LISTSTATS_FILE_SUFFIX: &'static str = ".ls.aget";

/// List statistic
///
/// `ListStats` struct records total and index two number.
/// All information is stored at a local file.
///
/// [total 8bit][index 8bit]
/// `total` is given by user, presenting as the real total number of items of a list.
pub struct ListStats {
    inner: File,
}

impl ListStats {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<ListStats> {
        let inner = File::new(path, true)?;
        Ok(ListStats { inner })
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

    pub fn index(&mut self) -> Result<u64> {
        let mut buf: [u8; 8] = [0; 8];
        self.inner.read(&mut buf, Some(SeekFrom::Start(8)))?;
        let cl = u8x8_to_u64(&buf);
        Ok(cl)
    }

    pub fn write_total(&mut self, total: u64) -> Result<()> {
        let buf = u64_to_u8x8(total);
        self.inner.write(&buf, Some(SeekFrom::Start(0)))?;
        Ok(())
    }

    pub fn write_index(&mut self, index: u64) -> Result<()> {
        let buf = u64_to_u8x8(index);
        self.inner.write(&buf, Some(SeekFrom::Start(8)))?;
        Ok(())
    }
}
