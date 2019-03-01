use std::path::Path;

use actix_web::http::Uri;

use term_size::dimensions;

use crate::error::{AgetError, ArgError, Result};

pub trait FindFilename {
    fn find_file_name(&self) -> Result<&str, AgetError>;
}

impl FindFilename for Uri {
    fn find_file_name(&self) -> Result<&str, AgetError> {
        let path = Path::new(self.path());
        if let Some(file_name) = path.file_name() {
            Ok(file_name.to_str().unwrap())
        } else {
            Err(AgetError::NoFilename)
        }
    }
}

pub fn terminal_width() -> u64 {
    dimensions().unwrap().0 as u64
}

const SIZES: [&'static str; 4] = ["B", "K", "M", "G"];

pub trait SizeOfFmt {
    fn sizeof_fmt(&self) -> String;
}

impl SizeOfFmt for u64 {
    fn sizeof_fmt(&self) -> String {
        let mut num = *self as f64;
        for s in SIZES.iter() {
            if num < 1024.0 {
                return format!("{:.1}{}", num, s);
            }
            num /= 1024.0;
        }
        return format!("{:.1}{}", num, "G");
    }
}

impl SizeOfFmt for f64 {
    fn sizeof_fmt(&self) -> String {
        let mut num = *self;
        for s in SIZES.iter() {
            if num < 1024.0 {
                return format!("{:.1}{}", num, s);
            }
            num /= 1024.0;
        }
        return format!("{:.1}{}", num, "G");
    }
}

pub trait LiteralSize {
    fn literal_size(&self) -> Result<u64, ArgError>;
}

impl LiteralSize for &str {
    fn literal_size(&self) -> Result<u64, ArgError> {
        let (num, unit) = self.split_at(self.len() - 1);
        if unit.parse::<u8>().is_err() {
            let mut num = num.parse::<u64>()?;
            for s in SIZES.iter() {
                if s.to_lowercase() == unit.to_lowercase() {
                    return Ok(num);
                } else {
                    num *= 1024;
                }
            }
            Ok(num)
        } else {
            let num = self.parse::<u64>()?;
            Ok(num)
        }
    }
}

pub trait TimeOfFmt {
    fn timeof_fmt(&self) -> String;
}

impl TimeOfFmt for u64 {
    fn timeof_fmt(&self) -> String {
        let mut num = *self as f64;
        if num < 60.0 {
            return format!("{:.0}s", num);
        }
        num /= 60.0;
        if num < 60.0 {
            return format!("{:.0}m", num);
        }
        num /= 60.0;
        if num < 24.0 {
            return format!("{:.0}h", num);
        }
        num /= 24.0;
        return format!("{:.0}d", num);
    }
}
