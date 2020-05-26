use std::{io::SeekFrom, path::Path};

use crate::common::{
    bytes::bytes::{u64_to_u8x8, u8x8_to_u64},
    errors::Result,
    file::File,
};

/// Byte array recorder
///
/// `ByteArrayRecorder` struct records a list u64/ numbers.
/// All information is stored at a local file.
///
/// [8bit][8bit][8bit]
/// `total` is given by user, presenting as the real total number of items of a list.
pub struct ByteArrayRecorder {
    inner: File,
}

impl ByteArrayRecorder {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<ByteArrayRecorder> {
        let inner = File::new(path, true)?;
        Ok(ByteArrayRecorder { inner })
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

    /// Read the index-th number
    pub fn index(&mut self, index: u64) -> Result<u64> {
        let mut buf: [u8; 8] = [0; 8];
        self.inner
            .read(&mut buf, Some(SeekFrom::Start(index * 8)))?;
        Ok(u8x8_to_u64(&buf))
    }

    pub fn write(&mut self, index: u64, num: u64) -> Result<()> {
        let buf = u64_to_u8x8(num);
        self.inner.write(&buf, Some(SeekFrom::Start(index * 8)))?;
        Ok(())
    }
}
