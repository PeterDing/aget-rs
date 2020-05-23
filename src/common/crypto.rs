use openssl::symm::{decrypt, Cipher};

use crate::common::errors::Result;

pub fn decrypt_aes128(key: &[u8], iv: &[u8], enc: &[u8]) -> Result<Vec<u8>> {
    let cipher = Cipher::aes_128_cbc();
    Ok(decrypt(cipher, key, Some(iv), enc)?)
}
