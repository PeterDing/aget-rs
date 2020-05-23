use std::{
    fs::{create_dir_all, metadata, remove_file, File as StdFile, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use crate::common::errors::{Error, Result};

/// File can be readed or writen only by opened.
pub struct File {
    path: PathBuf,
    file: Option<StdFile>,
    readable: bool,
}

impl File {
    pub fn new<P: AsRef<Path>>(path: P, readable: bool) -> Result<File> {
        let path = path.as_ref().to_path_buf();
        if path.is_dir() {
            return Err(Error::InvalidPath(format!("{:?}", path)));
        }

        Ok(File {
            path,
            file: None,
            readable,
        })
    }

    /// Create the dir if it does not exists
    fn create_dir<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        if !dir.as_ref().exists() {
            Ok(create_dir_all(dir)?)
        } else {
            Ok(())
        }
    }

    /// Create or open the file
    pub fn open(&mut self) -> Result<&mut Self> {
        if let Some(dir) = self.path.parent() {
            self.create_dir(dir)?;
        }
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

    pub fn file_name(&self) -> Option<&str> {
        if let Some(n) = self.path.as_path().file_name() {
            n.to_str()
        } else {
            None
        }
    }

    pub fn file(&mut self) -> Result<&mut StdFile> {
        if let Some(ref mut file) = self.file {
            Ok(file)
        } else {
            Err(Error::Bug("`store::File::file` must be opened".to_string()))
        }
    }

    pub fn size(&self) -> u64 {
        if let Ok(md) = metadata(&self.path) {
            md.len()
        } else {
            0
        }
    }

    pub fn write(&mut self, buf: &[u8], seek: Option<SeekFrom>) -> Result<usize> {
        if let Some(seek) = seek {
            self.seek(seek)?;
        }
        Ok(self.file()?.write(buf)?)
    }

    pub fn read(&mut self, buf: &mut [u8], seek: Option<SeekFrom>) -> Result<usize> {
        if let Some(seek) = seek {
            self.seek(seek)?;
        }
        Ok(self.file()?.read(buf)?)
    }

    pub fn seek(&mut self, seek: SeekFrom) -> Result<u64> {
        Ok(self.file()?.seek(seek)?)
    }

    pub fn set_len(&mut self, size: u64) -> Result<()> {
        Ok(self.file()?.set_len(size)?)
    }

    pub fn remove(&self) -> Result<()> {
        Ok(remove_file(self.path.as_path())?)
    }
}
