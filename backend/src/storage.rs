//! Shared S3 object-key builders and presigned-URL helpers used by both the
//! upload flow (multipart + thumbnail PUTs) and the download flow (presigned
//! GET). Centralised here so the key layout `files/{owner}/{file}/{variant}`
//! has a single source of truth.

use aws_sdk_s3::presigning::PresigningConfig;
use tracing::error;

use crate::error::{Error, Result};
use crate::file::File;

/// S3 key for the file's encrypted original (the segment-encrypted object).
pub(crate) fn s3_original_key(file: &File) -> String {
    format!("files/{}/{}/original", file.owner_id, file.id)
}

/// S3 key for a thumbnail variant (e.g. `512-cover`).
pub(crate) fn s3_thumbnail_key(file: &File, variant: &str) -> String {
    format!("files/{}/{}/{}", file.owner_id, file.id, variant)
}

/// Build a presigning config with the given lifetime.
pub(crate) fn presigning_config(ttl_secs: u64) -> Result<PresigningConfig> {
    PresigningConfig::expires_in(std::time::Duration::from_secs(ttl_secs)).map_err(|e| {
        error!(error = %e, "Invalid presigning configuration");
        Error::Storage
    })
}

/// Presign a `PutObject` URL (used for thumbnail uploads).
pub(crate) async fn presign_put_object(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    config: &PresigningConfig,
) -> Result<String> {
    let presigned = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .presigned(config.clone())
        .await
        .map_err(|e| {
            error!(key, error = %e, "Failed to presign PutObject");
            Error::Storage
        })?;

    Ok(presigned.uri().to_string())
}

/// Presign a `GetObject` URL. The returned URL serves arbitrary HTTP `Range`
/// requests (Range is not part of the SigV4 signature), which is what lets a
/// client fetch and decrypt individual segments for seeking.
pub(crate) async fn presign_get_object(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    config: &PresigningConfig,
) -> Result<String> {
    let presigned = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .presigned(config.clone())
        .await
        .map_err(|e| {
            error!(key, error = %e, "Failed to presign GetObject");
            Error::Storage
        })?;

    Ok(presigned.uri().to_string())
}
