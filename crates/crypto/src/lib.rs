use ::rsa::{Oaep, RsaPublicKey};
use aes_gcm::aead::consts::U12;
use aes_gcm::Nonce;
use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit};
use argon2::Argon2;
use base64ct::{Base64, Encoding};
use bytes::Bytes;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use error::Error;

pub mod error;
pub mod rsa;

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

pub fn decrypt_data(
    file_uuid: Uuid,
    encryption_key: &Key<Aes256Gcm>,
    encrypted_data: Bytes,
) -> error::Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(encryption_key);
    let nonce = generate_nonce_from_uuid(file_uuid);

    cipher
        .decrypt(&nonce, encrypted_data.as_ref())
        .map_err(|e| Error::EncryptionError(format!("Failed to decrypt data: {}", e)))
}

pub fn decode_encryption_key(
    key: &str,
    decrypt_fn: impl Fn(&[u8]) -> error::Result<Vec<u8>>,
) -> error::Result<Key<Aes256Gcm>> {
    let encryption_key = Base64::decode_vec(key)
        .map_err(|e| {
            Error::EncryptionError(format!("Could not decode encryption key, error {}", e))
        })
        .and_then(|key| decrypt_fn(&key))
        .map(|key| Key::<Aes256Gcm>::clone_from_slice(&key))?;
    Ok(encryption_key)
}

pub fn generate_encoded_encryption_key(public_key: &RsaPublicKey) -> String {
    let key: Key<Aes256Gcm> = Aes256Gcm::generate_key(OsRng);
    let encrypted_key = encrypt(&key, public_key);
    Base64::encode_string(&encrypted_key)
}

pub fn encrypt(data: &[u8], public_key: &RsaPublicKey) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let padding = Oaep::new::<Sha256>();
    public_key
        .encrypt(&mut rng, padding, data)
        .expect("failed to encrypt")
}

pub fn verify_data_hash(uuid: Uuid, sha256: &str, data: &Bytes) -> error::Result<()> {
    let data_hash = hash(data);
    if data_hash != sha256 {
        return Err(Error::EncryptionError(format!(
            "File {} hash mismatch, expected {}, got {}",
            uuid, sha256, data_hash
        )));
    }
    Ok(())
}

fn hash(data: &[u8]) -> String {
    let hash = Sha256::digest(data);

    Base64::encode_string(&hash)
}

fn generate_nonce_from_uuid(uuid: Uuid) -> Nonce<U12> {
    let uuid_bytes = uuid.as_bytes();
    let hash = Sha256::digest(uuid_bytes);
    let nonce_bytes = &hash[0..12];
    Nonce::clone_from_slice(nonce_bytes)
}

pub fn encrypt_data_raw(data: &[u8], cipher: &Aes256Gcm, nonce: &Nonce<U12>) -> String {
    let encrypted = cipher.encrypt(nonce, data).unwrap();
    Base64::encode_string(&encrypted)
}

pub fn decrypt_data_raw(
    data: &str,
    cipher: &Aes256Gcm,
    nonce: &Nonce<U12>,
) -> error::Result<Vec<u8>> {
    let decoded = Base64::decode_vec(data).unwrap();
    cipher
        .decrypt(nonce, decoded.as_ref())
        .map_err(|e| Error::EncryptionError(format!("Could not decrypt data, error: {}", e)))
}

pub fn generate_cipher(user_id: &Uuid, passphrase: &str) -> error::Result<(Aes256Gcm, Nonce<U12>)> {
    let salt = user_id.as_bytes();
    let nonce = &salt[4..16];
    let nonce = Nonce::clone_from_slice(nonce);

    let mut enc_key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut enc_key)
        .unwrap();
    let cipher = Aes256Gcm::new_from_slice(&enc_key).unwrap();

    Ok((cipher, nonce))
}
