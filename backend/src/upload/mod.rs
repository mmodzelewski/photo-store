mod handlers;
pub(crate) mod repository;
mod routes;

pub(crate) use handlers::cleanup_expired_uploads;
pub(crate) use routes::routes;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ulid::Id;

/// S3 minimum part size (except last part).
pub(crate) const MIN_CHUNK_SIZE: i64 = 5 * 1024 * 1024;

/// Maximum chunk size (our policy).
pub(crate) const MAX_CHUNK_SIZE: i64 = 100 * 1024 * 1024;

/// S3 maximum number of parts per multipart upload.
pub(crate) const MAX_PARTS: i64 = 10_000;

/// Required thumbnail variants for upload completion.
pub(crate) const REQUIRED_THUMBNAILS: &[&str] = &["512-cover", "1920-contain"];

#[derive(Debug, Clone)]
pub(crate) struct UploadSession {
    pub file_id: Id,
    pub upload_id: String,
    pub total_size: i64,
    pub chunk_size: i32,
    pub total_chunks: i32,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

impl From<crate::entity::upload_sessions::Model> for UploadSession {
    fn from(m: crate::entity::upload_sessions::Model) -> Self {
        UploadSession {
            file_id: Id::from(m.file_id),
            upload_id: m.upload_id,
            total_size: m.total_size,
            chunk_size: m.chunk_size,
            total_chunks: m.total_chunks,
            created_at: m.created_at,
            expires_at: m.expires_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct InitUploadRequest {
    pub total_size: i64,
    pub chunk_size: i32,
}

#[derive(Debug, Serialize)]
pub(super) struct ChunkUrl {
    pub part_number: i32,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ThumbnailUrl {
    pub variant: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub(super) struct InitUploadResponse {
    pub total_chunks: i32,
    pub chunk_size: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    pub chunk_urls: Vec<ChunkUrl>,
    pub thumbnail_urls: Vec<ThumbnailUrl>,
}

#[derive(Debug, Serialize)]
pub(super) struct UploadStatusResponse {
    pub total_chunks: i32,
    pub chunk_size: i32,
    pub total_size: i64,
    pub received: Vec<i32>,
    pub missing: Vec<ChunkUrl>,
    pub thumbnails_received: Vec<String>,
    pub thumbnails_missing: Vec<ThumbnailUrl>,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub(super) struct CompletePart {
    pub part_number: i32,
    pub etag: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct CompleteUploadRequest {
    pub parts: Vec<CompletePart>,
}

#[derive(Debug, Serialize)]
pub(super) struct CompleteUploadResponse {
    pub file_id: Id,
}
