use std::path::Path;

use crate::common::{
    errors::{Error, Result},
    net::net_type::Uri,
};

/// Use the last of components of uri as a file name
pub trait UriFileName {
    fn file_name(&self) -> Result<&str>;
}

impl UriFileName for Uri {
    fn file_name(&self) -> Result<&str> {
        let path = Path::new(self.path());
        if let Some(file_name) = path.file_name() {
            Ok(file_name.to_str().unwrap())
        } else {
            Err(Error::NoFilename)
        }
    }
}
