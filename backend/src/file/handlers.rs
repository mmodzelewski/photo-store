use axum::extract::Query;
use axum::{
    Json,
    extract::{Path, State},
};
use sdk::dtos::file::{DownloadUrlResponse, FileMetadata, FilesUploadRequest};
use sdk::media::MediaType;
use serde::{Deserialize, Deserializer, de};
use std::ops::RangeInclusive;
use time::{Duration, OffsetDateTime};
use tracing::{debug, error, warn};

use super::File;
use super::repository::DbFileRepository;
use crate::file::repository::FileRepository;
use crate::storage::{presign_get_object, presigning_config, s3_original_key, s3_thumbnail_key};
use crate::ulid::Id;
use crate::{
    AppState,
    error::{Error, Result},
    file::FileState,
    session::Session,
};

pub(super) async fn upload_files_metadata(
    State(state): State<AppState>,
    session: Session,
    Json(request): Json<FilesUploadRequest>,
) -> Result<()> {
    debug!(count = request.files.len(), "Uploading files metadata",);

    let segment_bounds =
        state.config.upload.min_segment_size..=state.config.upload.max_segment_size;
    let mut repo = DbFileRepository { db: state.db };
    upload_files_metadata_internal(&mut repo, session, request, segment_bounds).await?;

    Ok(())
}

/// Validate the client-supplied media metadata. The server can't inspect the
/// ciphertext, so these are the only guard rails before persisting.
fn validate_metadata(item: &FileMetadata, segment_bounds: &RangeInclusive<u32>) -> Result<()> {
    if item.content_type.trim().is_empty() {
        error!("Rejecting file with empty content_type");
        return Err(Error::FileUpload);
    }
    if item.plaintext_size == 0 {
        error!("Rejecting file with zero plaintext_size");
        return Err(Error::FileUpload);
    }
    if item.width == 0 || item.height == 0 {
        error!(
            width = item.width,
            height = item.height,
            "Rejecting file with zero dimensions"
        );
        return Err(Error::FileUpload);
    }
    if !segment_bounds.contains(&item.segment_size) {
        error!(
            segment_size = item.segment_size,
            "segment_size out of range"
        );
        return Err(Error::FileUpload);
    }
    if item.enc_scheme != sdk::crypto::ENC_SCHEME_SEGMENTED {
        error!(
            enc_scheme = item.enc_scheme,
            "Unsupported encryption scheme"
        );
        return Err(Error::FileUpload);
    }
    if item.media_type == MediaType::Video && item.duration_ms.is_none() {
        error!("Rejecting video without duration_ms");
        return Err(Error::FileUpload);
    }
    Ok(())
}

async fn upload_files_metadata_internal(
    repo: &mut impl FileRepository,
    session: Session,
    request: FilesUploadRequest,
    segment_bounds: RangeInclusive<u32>,
) -> Result<()> {
    let request_user_id: Id = request.user_id.into();
    if request_user_id != session.user_id() {
        error!("Upload authorization mismatch");
        return Err(Error::Forbidden);
    }

    for item in request.files {
        let file_id: Id = item.id.into();
        let exists = repo.exists(&file_id).await?;

        if exists {
            warn!(%file_id, "File already exists, skipping upload");
            continue;
        }

        validate_metadata(&item, &segment_bounds)?;

        let file = File {
            id: file_id,
            path: item.path.clone(),
            name: item.path.split('/').next_back().unwrap().to_string(),
            state: FileState::New,
            created_at: item.date,
            added_at: OffsetDateTime::now_utc(),
            sha256: item.sha256.clone(),
            owner_id: request_user_id,
            uploader_id: session.user_id(),
            enc_key: item.key.clone(),
            media_type: item.media_type,
            content_type: item.content_type.clone(),
            width: item.width,
            height: item.height,
            duration_ms: item.duration_ms,
            segment_size: item.segment_size,
            plaintext_size: item.plaintext_size,
            nonce_salt: item.nonce_salt,
            enc_scheme: item.enc_scheme,
        };

        debug!("Saving file metadata");
        repo.save(&file).await?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct DownloadParams {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    from: Option<OffsetDateTime>,
}

fn empty_string_as_none<'de, D: Deserializer<'de>>(
    de: D,
) -> std::result::Result<Option<OffsetDateTime>, D::Error> {
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(value) => {
            let timestamp = value.parse::<i64>().map_err(|_err| {
                de::Error::invalid_value(de::Unexpected::Str(value), &"a unix timestamp in seconds")
            })?;
            OffsetDateTime::from_unix_timestamp(timestamp)
                .map(Some)
                .map_err(|err| de::Error::invalid_value(de::Unexpected::Signed(timestamp), &err))
        }
    }
}

pub(super) async fn get_files_metadata(
    State(state): State<AppState>,
    Query(params): Query<DownloadParams>,
    session: Session,
) -> Result<Json<Vec<FileMetadata>>> {
    debug!("Getting files metadata");

    let repo = DbFileRepository { db: state.db };
    let files = repo
        .find_synced_files(&session.user_id(), params.from)
        .await?;

    let metadata: Vec<FileMetadata> = files
        .into_iter()
        .map(|file| FileMetadata {
            path: file.path,
            id: file.id.into(),
            date: file.created_at,
            sha256: file.sha256,
            key: file.enc_key,
            media_type: file.media_type,
            content_type: file.content_type,
            width: file.width,
            height: file.height,
            duration_ms: file.duration_ms,
            segment_size: file.segment_size,
            plaintext_size: file.plaintext_size,
            nonce_salt: file.nonce_salt,
            enc_scheme: file.enc_scheme,
        })
        .collect();

    Ok(Json(metadata))
}

#[derive(Debug, Deserialize)]
pub(super) struct FileDownloadParams {
    variant: Option<sdk::thumbnails::ThumbnailVariant>,
}

/// Issue a short-lived presigned S3 GET URL for the requested object. The
/// client fetches the (encrypted) bytes directly from storage — supporting
/// HTTP Range so it can pull and decrypt individual segments to seek — rather
/// than streaming them through the backend.
pub(super) async fn download_file(
    State(state): State<AppState>,
    session: Session,
    Path(file_id): Path<Id>,
    Query(params): Query<FileDownloadParams>,
) -> Result<Json<DownloadUrlResponse>> {
    debug!(%file_id, variant = ?params.variant, "Issuing download URL");
    let repo = DbFileRepository { db: state.db };

    let file = repo.find(&file_id).await?.ok_or(Error::FileNotFound)?;

    if file.owner_id != session.user_id() {
        return Err(Error::Forbidden);
    }

    let key = match &params.variant {
        Some(variant) => s3_thumbnail_key(&file, &variant.to_string()),
        None => s3_original_key(&file),
    };

    let ttl_secs = state.config.upload.presigned_url_ttl_secs;
    let presigning = presigning_config(ttl_secs)?;
    let url = presign_get_object(
        &state.s3_client,
        &state.config.storage.bucket_name,
        &key,
        &presigning,
    )
    .await?;
    let expires_at = OffsetDateTime::now_utc() + Duration::seconds(ttl_secs as i64);

    Ok(Json(DownloadUrlResponse { url, expires_at }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::repository::tests::InMemoryFileRepository;
    use time::OffsetDateTime;

    /// A wide bound so tests don't depend on production defaults.
    fn any_segment_size() -> RangeInclusive<u32> {
        1..=u32::MAX
    }

    fn image_metadata(id: ulid::Ulid, path: &str) -> FileMetadata {
        FileMetadata {
            id,
            path: path.to_string(),
            date: OffsetDateTime::now_utc(),
            sha256: "sha256".to_string(),
            key: "key".to_string(),
            media_type: MediaType::Image,
            content_type: "image/jpeg".to_string(),
            width: 640,
            height: 480,
            duration_ms: None,
            segment_size: sdk::segment::DEFAULT_SEGMENT_SIZE,
            plaintext_size: 4096,
            nonce_salt: 42,
            enc_scheme: sdk::crypto::ENC_SCHEME_SEGMENTED,
        }
    }

    #[tokio::test]
    async fn uploading_for_another_user_should_return_an_error() {
        // given
        let mut repo = InMemoryFileRepository::new();
        let request = FilesUploadRequest {
            user_id: ulid::Ulid::new(),
            files: vec![],
        };

        let session = Session::new(Id::new());

        // when
        let result =
            upload_files_metadata_internal(&mut repo, session, request, any_segment_size()).await;

        // then
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Forbidden));
    }

    #[tokio::test]
    async fn should_save_metadata_in_repository() {
        // given
        let mut repo = InMemoryFileRepository::new();
        let user_id = ulid::Ulid::new();
        let file_id = ulid::Ulid::new();
        let request = FilesUploadRequest {
            user_id,
            files: vec![image_metadata(file_id, "/home/pics/test.jpg")],
        };
        let metadata = request.files[0].clone();
        let session = Session::new(user_id.into());

        // when
        upload_files_metadata_internal(&mut repo, session, request, any_segment_size())
            .await
            .unwrap();

        // then
        let files = repo.files.borrow();
        let file = &files[0];
        assert_eq!(file.path, "/home/pics/test.jpg");
        assert_eq!(file.name, "test.jpg");
        assert!(matches!(file.state, FileState::New));
        assert_eq!(file.id, file_id.into());
        assert_eq!(file.created_at, metadata.date);
        assert_eq!(file.sha256, "sha256");
        assert_eq!(file.owner_id, user_id.into());
        assert_eq!(file.uploader_id, user_id.into());
        assert_eq!(file.enc_key, "key");
        assert_eq!(file.media_type, MediaType::Image);
        assert_eq!(file.content_type, "image/jpeg");
        assert_eq!(file.segment_size, sdk::segment::DEFAULT_SEGMENT_SIZE);
        assert_eq!(file.plaintext_size, 4096);
        assert_eq!(file.nonce_salt, 42);
        assert_eq!(file.enc_scheme, sdk::crypto::ENC_SCHEME_SEGMENTED);
    }

    #[tokio::test]
    async fn rejects_video_without_duration() {
        let mut repo = InMemoryFileRepository::new();
        let user_id = ulid::Ulid::new();
        let mut item = image_metadata(ulid::Ulid::new(), "/clips/movie.mp4");
        item.media_type = MediaType::Video;
        item.content_type = "video/mp4".to_string();
        item.duration_ms = None;
        let request = FilesUploadRequest {
            user_id,
            files: vec![item],
        };
        let session = Session::new(user_id.into());

        let result =
            upload_files_metadata_internal(&mut repo, session, request, any_segment_size()).await;

        assert!(matches!(result.unwrap_err(), Error::FileUpload));
        assert!(repo.files.borrow().is_empty());
    }

    #[tokio::test]
    async fn rejects_segment_size_out_of_bounds() {
        let mut repo = InMemoryFileRepository::new();
        let user_id = ulid::Ulid::new();
        let mut item = image_metadata(ulid::Ulid::new(), "/home/pics/big.jpg");
        item.segment_size = 64;
        let request = FilesUploadRequest {
            user_id,
            files: vec![item],
        };
        let session = Session::new(user_id.into());

        // bounds that exclude 64
        let result = upload_files_metadata_internal(&mut repo, session, request, 1024..=4096).await;

        assert!(matches!(result.unwrap_err(), Error::FileUpload));
        assert!(repo.files.borrow().is_empty());
    }
}
