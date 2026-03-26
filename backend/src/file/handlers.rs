use axum::body::Bytes;
use axum::extract::Query;
use axum::http::{HeaderMap, HeaderValue};
use axum::{
    Json,
    extract::{Path, State},
};
use dtos::file::{FileMetadata, FilesUploadRequest};
use http::header;
use serde::{Deserialize, Deserializer, de};
use time::OffsetDateTime;
use tracing::{debug, error, warn};

use super::File;
use super::repository::DbFileRepository;
use crate::file::repository::FileRepository;
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

    let mut repo = DbFileRepository { db: state.db };
    upload_files_metadata_internal(&mut repo, session, request).await?;

    Ok(())
}

async fn upload_files_metadata_internal(
    repo: &mut impl FileRepository,
    session: Session,
    request: FilesUploadRequest,
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
        })
        .collect();

    Ok(Json(metadata))
}

const ORIGINAL: &str = "original";

#[derive(Debug, Deserialize)]
pub(super) struct FileDownloadParams {
    variant: Option<sdk::thumbnails::ThumbnailVariant>,
}

pub(super) async fn download_file(
    State(state): State<AppState>,
    session: Session,
    Path(file_id): Path<Id>,
    Query(params): Query<FileDownloadParams>,
) -> Result<(HeaderMap, Bytes)> {
    debug!(%file_id, variant = ?params.variant, "Downloading file");
    let repo = DbFileRepository { db: state.db };

    let file = repo.find(&file_id).await?.ok_or(Error::FileNotFound)?;

    if file.owner_id != session.user_id() {
        return Err(Error::Forbidden);
    }

    let variant = params
        .variant
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| ORIGINAL.to_owned());

    let file_key = format!("files/{}/{}/{}", file.owner_id, file.id, variant);

    let get_object_output = state
        .s3_client
        .get_object()
        .bucket(&state.config.storage.bucket_name)
        .key(&file_key)
        .send()
        .await
        .map_err(|e| {
            error!(%file_id, error = %e, "Could not get file from storage");
            Error::Storage
        })?;

    let content_type = get_object_output
        .content_type()
        .unwrap_or("application/octet-stream");

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type).map_err(|e| {
            error!(content_type, error = %e, "Could not parse content type");
            Error::FileDownload
        })?,
    );

    let data = get_object_output
        .body
        .collect()
        .await
        .map_err(|e| {
            error!(%file_id, error = %e, "Could not read file bytes from storage");
            Error::FileDownload
        })?
        .into_bytes();

    Ok((headers, data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::repository::tests::InMemoryFileRepository;
    use time::OffsetDateTime;

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
        let result = upload_files_metadata_internal(&mut repo, session, request).await;

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
            files: vec![FileMetadata {
                path: "/home/pics/test.jpg".to_string(),
                id: file_id,
                date: OffsetDateTime::now_utc(),
                sha256: "sha256".to_string(),
                key: "key".to_string(),
            }],
        };
        let metadata = request.files[0].clone();
        let session = Session::new(user_id.into());

        // when
        upload_files_metadata_internal(&mut repo, session, request)
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
    }
}
