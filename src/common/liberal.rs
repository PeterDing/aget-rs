use crate::common::errors::{Error, Result};

const SIZES: [&'static str; 5] = ["B", "K", "M", "G", "T"];

/// Convert liberal number to u64
/// e.g.
/// 100k -> 100 * 1024
pub trait ParseLiteralNumber {
    fn literal_number(&self) -> Result<u64, Error>;
}

impl ParseLiteralNumber for &str {
    fn literal_number(&self) -> Result<u64, Error> {
        let (num, unit) = self.split_at(self.len() - 1);
        if unit.parse::<u8>().is_err() {
            let mut num = num.parse::<u64>()?;
            for s in &SIZES {
                if s == &unit.to_uppercase() {
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

/// Convert seconds to date format
pub trait ToDate {
    fn date(&self) -> String;
}

impl ToDate for u64 {
    fn date(&self) -> String {
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
