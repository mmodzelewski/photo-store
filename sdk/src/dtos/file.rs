use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::media::MediaType;

#[derive(Debug, Serialize, Deserialize)]
pub struct FilesUploadRequest {
    pub user_id: ulid::Ulid,
    pub files: Vec<FileMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileMetadata {
    pub id: ulid::Ulid,
    pub path: String,
    #[serde(with = "time::serde::iso8601")]
    pub date: OffsetDateTime,
    pub sha256: String,
    pub key: String,

    /// What kind of media this is. Client-supplied (the server can't inspect
    /// the ciphertext) and drives the required-variant contract + library UI.
    pub media_type: MediaType,
    /// Original MIME type, applied by the client after decryption (the stored
    /// S3 object is opaque ciphertext, served as `application/octet-stream`).
    pub content_type: String,
    /// Pixel dimensions. Required for both images and video: the client must
    /// decode the media to produce the required poster thumbnails, so the
    /// dimensions are always known at upload time.
    pub width: u32,
    pub height: u32,
    /// Duration in milliseconds for video; `None` for images.
    pub duration_ms: Option<u32>,

    /// Plaintext bytes per encryption segment (see [`crate::segment`]).
    pub segment_size: u32,
    /// Original (plaintext) size in bytes, used to derive the segment layout
    /// for seeking. The uploaded ciphertext size is larger by `count * 16`.
    pub plaintext_size: u64,
    /// Per-file random salt used to derive segment nonces.
    pub nonce_salt: u32,
    /// Encryption scheme version (see [`crate::crypto::ENC_SCHEME_SEGMENTED`]).
    pub enc_scheme: u8,
}

/// Response handed back for a download request: a short-lived presigned S3 GET
/// URL the client fetches directly (supporting HTTP Range for seeking).
#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadUrlResponse {
    pub url: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}
