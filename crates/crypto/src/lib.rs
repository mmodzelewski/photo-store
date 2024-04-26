use aes_gcm::aead::consts::U12;
use aes_gcm::Nonce;
use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit};
use base64ct::{Base64, Encoding};
use bytes::Bytes;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use error::Error;

pub mod error;

pub trait CryptoFileDesc {
    fn uuid(&self) -> Uuid;
    fn key(&self) -> Option<&str>;
    fn sha256(&self) -> &str;
}

pub fn encrypt_data<File: CryptoFileDesc>(
    file: &File,
    data: Bytes,
) -> error::Result<(Vec<u8>, String)> {
    let encryption_key = decode_encryption_key(file)?;
    let aes256key = Key::<Aes256Gcm>::from_slice(&encryption_key);
    let cipher = Aes256Gcm::new(aes256key);
    let nonce = generate_nonce_from_uuid(file.uuid());

    let encrypted_data = cipher.encrypt(&nonce, data.as_ref()).unwrap();
    let data_hash = hash(&encrypted_data);
    Ok((encrypted_data, data_hash))
}

fn decode_encryption_key<File: CryptoFileDesc>(file: &File) -> error::Result<Vec<u8>> {
    let encryption_key = file
        .key()
        .as_ref()
        .ok_or(Error::EncryptionError(format!(
            "Missing encryption key for file {}",
            file.uuid()
        )))
        .and_then(|k| {
            Base64::decode_vec(k).map_err(|e| {
                Error::EncryptionError(format!(
                    "Could not decode encryption key for file {}, error {}",
                    file.uuid(),
                    e
                ))
            })
        })?;
    Ok(encryption_key)
}

pub fn verify_data_hash(uuid: Uuid, sha256: &str, data: &Bytes) -> error::Result<()> {
    let data_hash = hash(&data);
    if data_hash != sha256 {
        return Err(Error::EncryptionError(format!(
            "File {} hash mismatch, expected {}, got {}",
            uuid, sha256, data_hash
        )));
    }
    return Ok(());
}

fn hash(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    let encoded = Base64::encode_string(&hash);
    return encoded;
}

fn generate_nonce_from_uuid(uuid: Uuid) -> Nonce<U12> {
    let uuid_bytes = uuid.as_bytes();
    let hash = Sha256::digest(uuid_bytes);
    let nonce_bytes = &hash[0..12];
    Nonce::clone_from_slice(nonce_bytes)
}
