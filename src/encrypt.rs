use crate::error::Error;
use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
use base64::Engine;
use sha3::Digest;

pub(crate) struct Sha256;
impl Sha256 {
    pub fn digest(data: &str) -> Vec<u8> {
        sha3::Sha3_256::digest(data.as_bytes()).to_vec()
    }
}

static AES_NONCE_SIZE: usize = 12;

pub(crate) struct Aes;
impl Aes {
    pub fn encrypt(plaintext: &str, key: &str) -> Result<String, Error> {
        let key_hash = Sha256::digest(key);
        let key = Key::<Aes256Gcm>::from_slice(&key_hash);

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let cipher = Aes256Gcm::new(key);
        let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes()).map_err(Error::from)?;

        let mut encrypted_data: Vec<u8> = nonce.to_vec();
        encrypted_data.extend_from_slice(&ciphertext);

        Ok(base64::prelude::BASE64_STANDARD.encode(&encrypted_data))
    }

    pub fn decrypt(data: &str, key: &str) -> Result<String, Error> {
        let ciphertext = base64::prelude::BASE64_STANDARD.decode(data.as_bytes()).map_err(Error::from)?;

        let key_hash = Sha256::digest(key);
        let key = Key::<Aes256Gcm>::from_slice(&key_hash);

        let (nonce_arr, ciphered_data) = ciphertext.split_at(AES_NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_arr);

        let cipher = Aes256Gcm::new(key);
        let plaintext = cipher.decrypt(nonce, ciphered_data).map_err(Error::from)?;

        String::from_utf8(plaintext).map_err(Error::from)
    }
}
