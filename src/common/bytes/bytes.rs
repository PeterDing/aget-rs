use std::num::ParseIntError;

use crate::common::errors::Result;

/// Create an integer value from its representation as a byte array in big endian.
pub fn u8x8_to_u64(u8x8: &[u8; 8]) -> u64 {
    u64::from_be_bytes(*u8x8)
}

/// Return the memory representation of this integer as a byte array in big-endian (network) byte
/// order.
pub fn u64_to_u8x8(u: u64) -> [u8; 8] {
    u.to_be_bytes()
}

pub fn u32_to_u8x4(u: u32) -> [u8; 4] {
    u.to_be_bytes()
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}
