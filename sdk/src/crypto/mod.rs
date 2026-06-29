use ::rsa::{Oaep, RsaPublicKey};
use aes_gcm::Nonce;
use aes_gcm::aead::Payload;
use aes_gcm::aead::consts::U12;
use aes_gcm::{
    Aes256Gcm, Key, KeyInit,
    aead::{Aead, Generate},
};
use argon2::Argon2;
use base64ct::{Base64, Encoding};
use bytes::Bytes;
use sha2::{Digest, Sha256};
use ulid::Ulid;

use self::error::Error;

pub mod error;
pub mod rsa;

/// Encryption scheme version for per-segment AES-256-GCM. Bound into every
/// segment's AAD so an attacker can't downgrade to (or forge) another scheme,
/// and reserved as the hook for future crypto agility.
pub const ENC_SCHEME_SEGMENTED: u8 = 2;

pub trait CryptoFileDesc {
    fn id(&self) -> Ulid;
    fn sha256(&self) -> &str;
}

/// 96-bit GCM nonce for a segment: a per-file random `salt` (32 bits) followed
/// by the segment index as a 64-bit counter. The counter guarantees per-key
/// nonce uniqueness across segments; the salt guards against (key, nonce)
/// reuse if a file id were ever re-sealed with different plaintext.
fn segment_nonce(salt: u32, seg_index: u64) -> Nonce<U12> {
    let mut bytes = [0u8; 12];
    bytes[0..4].copy_from_slice(&salt.to_be_bytes());
    bytes[4..12].copy_from_slice(&seg_index.to_be_bytes());
    Nonce::from(bytes)
}

/// Additional authenticated data bound to each segment. The manifest
/// (`seg_index`, `seg_count`) is server-stored plaintext the client doesn't
/// trust; binding it here means GCM authentication fails on any reorder,
/// splice, truncation, or scheme downgrade.
fn segment_aad(scheme: u8, file_id: Ulid, seg_index: u64, seg_count: u64) -> [u8; 33] {
    let mut aad = [0u8; 33];
    aad[0] = scheme;
    aad[1..17].copy_from_slice(&file_id.to_bytes());
    aad[17..25].copy_from_slice(&seg_index.to_be_bytes());
    aad[25..33].copy_from_slice(&seg_count.to_be_bytes());
    aad
}

/// Seal one plaintext segment. Output is the segment ciphertext followed by the
/// 16-byte GCM tag; concatenate outputs in index order to form the object.
pub fn encrypt_segment(
    file: &impl CryptoFileDesc,
    key: &Key<Aes256Gcm>,
    salt: u32,
    scheme: u8,
    seg_index: u64,
    seg_count: u64,
    plaintext: &[u8],
) -> error::Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key);
    let nonce = segment_nonce(salt, seg_index);
    let aad = segment_aad(scheme, file.id(), seg_index, seg_count);
    cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|e| Error::EncryptionError(format!("Failed to encrypt segment {seg_index}: {e}")))
}

/// Open one segment sealed by [`encrypt_segment`]. The `salt`, `scheme`,
/// `seg_index`, and `seg_count` must match what the segment was sealed with, or
/// authentication fails.
pub fn decrypt_segment(
    file_id: Ulid,
    key: &Key<Aes256Gcm>,
    salt: u32,
    scheme: u8,
    seg_index: u64,
    seg_count: u64,
    ciphertext: &[u8],
) -> error::Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key);
    let nonce = segment_nonce(salt, seg_index);
    let aad = segment_aad(scheme, file_id, seg_index, seg_count);
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad: &aad,
            },
        )
        .map_err(|e| Error::EncryptionError(format!("Failed to decrypt segment {seg_index}: {e}")))
}

pub fn decode_encryption_key(
    key: &str,
    decrypt_fn: impl Fn(&[u8]) -> error::Result<Vec<u8>>,
) -> error::Result<Key<Aes256Gcm>> {
    let wrapped = Base64::decode_vec(key).map_err(|e| {
        Error::EncryptionError(format!("Could not decode encryption key, error {}", e))
    })?;
    let raw = decrypt_fn(&wrapped)?;
    let bytes: [u8; 32] = raw.as_slice().try_into().map_err(|_| {
        Error::EncryptionError(format!("Decrypted key must be 32 bytes, got {}", raw.len()))
    })?;
    Ok(Key::<Aes256Gcm>::from(bytes))
}

pub fn generate_encoded_encryption_key(public_key: &RsaPublicKey) -> String {
    let key: Key<Aes256Gcm> = Key::<Aes256Gcm>::generate();
    let encrypted_key = encrypt(&key, public_key);
    Base64::encode_string(&encrypted_key)
}

pub fn encrypt(data: &[u8], public_key: &RsaPublicKey) -> Vec<u8> {
    let mut rng = rand::rng();
    let padding = Oaep::<Sha256>::new();
    public_key
        .encrypt(&mut rng, padding, data)
        .expect("failed to encrypt")
}

pub fn verify_data_hash(id: Ulid, sha256: &str, data: &Bytes) -> error::Result<()> {
    let data_hash = hash(data);
    if data_hash != sha256 {
        return Err(Error::EncryptionError(format!(
            "File {} hash mismatch, expected {}, got {}",
            id, sha256, data_hash
        )));
    }
    Ok(())
}

fn hash(data: &[u8]) -> String {
    let hash = Sha256::digest(data);

    Base64::encode_string(&hash)
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

pub fn generate_cipher(user_id: &Ulid, passphrase: &str) -> error::Result<(Aes256Gcm, Nonce<U12>)> {
    let salt = user_id.to_bytes();
    let nonce_bytes: [u8; 12] = salt[4..16].try_into().unwrap();
    let nonce = Nonce::from(nonce_bytes);

    let mut enc_key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), &salt, &mut enc_key)
        .unwrap();
    let cipher = Aes256Gcm::new_from_slice(&enc_key).unwrap();

    Ok((cipher, nonce))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment::{GCM_TAG_SIZE, SegmentLayout};

    struct TestFile {
        id: Ulid,
    }

    impl CryptoFileDesc for TestFile {
        fn id(&self) -> Ulid {
            self.id
        }
        fn sha256(&self) -> &str {
            ""
        }
    }

    fn key() -> Key<Aes256Gcm> {
        Key::<Aes256Gcm>::generate()
    }

    const SCHEME: u8 = ENC_SCHEME_SEGMENTED;

    #[test]
    fn segment_roundtrip() {
        let file = TestFile { id: Ulid::new() };
        let key = key();
        let plaintext = b"the quick brown fox";

        let ct = encrypt_segment(&file, &key, 7, SCHEME, 0, 1, plaintext).unwrap();
        // ciphertext carries a 16-byte tag
        assert_eq!(ct.len() as u64, plaintext.len() as u64 + GCM_TAG_SIZE);

        let pt = decrypt_segment(file.id, &key, 7, SCHEME, 0, 1, &ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn full_file_roundtrip_via_layout() {
        let file = TestFile { id: Ulid::new() };
        let key = key();
        let salt = 0xdead_beef;
        let data: Vec<u8> = (0..25u8).collect();
        let layout = SegmentLayout::new(10, data.len() as u64).unwrap();
        let count = layout.segment_count();

        // seal each segment and concatenate
        let mut object = Vec::new();
        for i in 0..count {
            let start = (i * layout.segment_size() as u64) as usize;
            let end = start + layout.plaintext_len(i) as usize;
            let ct =
                encrypt_segment(&file, &key, salt, SCHEME, i, count, &data[start..end]).unwrap();
            assert_eq!(ct.len() as u64, layout.ciphertext_len(i));
            object.extend_from_slice(&ct);
        }
        assert_eq!(object.len() as u64, layout.ciphertext_size());

        // open the middle segment by slicing the ciphertext object
        let i = 1;
        let off = layout.ciphertext_offset(i) as usize;
        let len = layout.ciphertext_len(i) as usize;
        let pt = decrypt_segment(
            file.id,
            &key,
            salt,
            SCHEME,
            i,
            count,
            &object[off..off + len],
        )
        .unwrap();
        assert_eq!(pt, &data[10..20]);
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let file = TestFile { id: Ulid::new() };
        let key = key();
        let mut ct = encrypt_segment(&file, &key, 1, SCHEME, 0, 3, b"hello").unwrap();
        ct[0] ^= 0xff;
        assert!(decrypt_segment(file.id, &key, 1, SCHEME, 0, 3, &ct).is_err());
    }

    #[test]
    fn wrong_index_fails() {
        let file = TestFile { id: Ulid::new() };
        let key = key();
        let ct = encrypt_segment(&file, &key, 1, SCHEME, 2, 5, b"hello").unwrap();
        // reorder attack: decrypt as if it were a different segment
        assert!(decrypt_segment(file.id, &key, 1, SCHEME, 3, 5, &ct).is_err());
    }

    #[test]
    fn wrong_count_fails() {
        let file = TestFile { id: Ulid::new() };
        let key = key();
        let ct = encrypt_segment(&file, &key, 1, SCHEME, 0, 5, b"hello").unwrap();
        // truncation attack: claim a different total segment count
        assert!(decrypt_segment(file.id, &key, 1, SCHEME, 0, 4, &ct).is_err());
    }

    #[test]
    fn wrong_scheme_fails() {
        let file = TestFile { id: Ulid::new() };
        let key = key();
        let ct = encrypt_segment(&file, &key, 1, SCHEME, 0, 1, b"hello").unwrap();
        // downgrade attack: claim a different scheme version
        assert!(decrypt_segment(file.id, &key, 1, 1, 0, 1, &ct).is_err());
    }

    #[test]
    fn wrong_file_id_fails() {
        let file = TestFile { id: Ulid::new() };
        let key = key();
        let ct = encrypt_segment(&file, &key, 1, SCHEME, 0, 1, b"hello").unwrap();
        // cross-file splice: decrypt under a different file id
        assert!(decrypt_segment(Ulid::new(), &key, 1, SCHEME, 0, 1, &ct).is_err());
    }
}
