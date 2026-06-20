use aes::cipher::{block_padding::Pkcs7, BlockModeDecrypt, BlockModeEncrypt, KeyIvInit};

use crate::common::errors::Result;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

pub fn encrypt_aes128(key: &[u8], iv: &[u8], buf: &[u8]) -> Vec<u8> {
    let cipher = Aes128CbcEnc::new_from_slices(key, iv).unwrap();
    cipher.encrypt_padded_vec::<Pkcs7>(buf)
}

pub fn decrypt_aes128(key: &[u8], iv: &[u8], buf: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes128CbcDec::new_from_slices(key, iv).unwrap();
    Ok(cipher.decrypt_padded_vec::<Pkcs7>(buf)?)
}
