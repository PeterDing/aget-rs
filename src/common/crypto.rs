use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};

use crate::common::errors::Result;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

pub fn decrypt_aes128(key: &[u8], iv: &[u8], enc: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes128CbcDec::new(key.into(), iv.into());
    Ok(cipher.decrypt_padded_vec_mut::<Pkcs7>(enc)?)
}
