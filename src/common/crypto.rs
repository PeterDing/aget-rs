use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

use crate::common::errors::Result;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

pub fn encrypt_aes128(key: &[u8], iv: &[u8], buf: &[u8]) -> Vec<u8> {
    let cipher = Aes128CbcEnc::new(key.into(), iv.into());
    cipher.encrypt_padded_vec_mut::<Pkcs7>(buf)
}

pub fn decrypt_aes128(key: &[u8], iv: &[u8], buf: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes128CbcDec::new(key.into(), iv.into());
    Ok(cipher.decrypt_padded_vec_mut::<Pkcs7>(buf)?)
}
