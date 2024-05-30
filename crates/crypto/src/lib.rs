use aes_gcm::aead::consts::U12;
use aes_gcm::Nonce;
use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit};
use base64ct::{Base64, Encoding};
use bytes::Bytes;
use rand::rngs::OsRng;
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use error::Error;

pub mod error;

pub trait CryptoFileDesc {
    fn uuid(&self) -> Uuid;
    fn sha256(&self) -> &str;
}

pub fn encrypt_data<File: CryptoFileDesc>(
    file: &File,
    encryption_key: &Key<Aes256Gcm>,
    data: Bytes,
) -> error::Result<(Vec<u8>, String)> {
    let cipher = Aes256Gcm::new(encryption_key);
    let nonce = generate_nonce_from_uuid(file.uuid());

    let encrypted_data = cipher.encrypt(&nonce, data.as_ref()).unwrap();
    let data_hash = hash(&encrypted_data);
    Ok((encrypted_data, data_hash))
}

pub fn decode_encryption_key<File: CryptoFileDesc>(
    key: &str,
    private_key: &RsaPrivateKey,
    file: &File,
) -> error::Result<Key<Aes256Gcm>> {
    let encryption_key = Base64::decode_vec(key)
        .map_err(|e| {
            Error::EncryptionError(format!(
                "Could not decode encryption key for file {}, error {}",
                file.uuid(),
                e
            ))
        })
        .and_then(|key| {
            let padding = Oaep::new::<Sha256>();
            private_key.decrypt(padding, &key).map_err(|e| {
                Error::EncryptionError(format!(
                    "Could not decrypt encryption key for file {}, error {}",
                    file.uuid(),
                    e
                ))
            })
        })
        .map(|key| Key::<Aes256Gcm>::clone_from_slice(&key))?;
    Ok(encryption_key)
}

pub fn generate_encoded_encryption_key(public_key: &RsaPublicKey) -> String {
    let key: Key<Aes256Gcm> = Aes256Gcm::generate_key(OsRng);
    let encrypted_key = encrypt(&key, public_key);
    Base64::encode_string(&encrypted_key)
}

pub fn generate_rsa_key() -> RsaPrivateKey {
    let mut rng = rand::thread_rng();

    let bits = 2048;
    RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key")
}

pub fn encrypt(data: &[u8], public_key: &RsaPublicKey) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let padding = Oaep::new::<Sha256>();
    public_key
        .encrypt(&mut rng, padding, data)
        .expect("failed to encrypt")
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
