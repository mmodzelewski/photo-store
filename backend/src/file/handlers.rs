use aws_sdk_s3::primitives::ByteStream;
use axum::body::Bytes;
use axum::extract::Query;
use axum::http::{HeaderMap, HeaderValue};
use axum::{
    Json,
    extract::{Multipart, Path, State, multipart::Field},
};
use dtos::file::{FileMetadata, FilesUploadRequest};
use http::header;
use serde::{Deserialize, Deserializer, de};
use time::OffsetDateTime;
use tracing::{debug, error, warn};

use super::{File, repository::DbFileRepository};
use crate::file::repository::FileRepository;
use crate::ulid::Id;
use crate::{
    AppState,
    ctx::Ctx,
    error::{Error, Result},
    file::FileState,
};

pub(super) async fn upload_files_metadata(
    State(state): State<AppState>,
    ctx: Ctx,
    Json(request): Json<FilesUploadRequest>,
) -> Result<()> {
    debug!(count = request.files.len(), "Uploading files metadata",);

    let mut repo = DbFileRepository { db: state.db };
    upload_files_metadata_internal(&mut repo, ctx, request).await?;

    Ok(())
}

async fn upload_files_metadata_internal(
    repo: &mut impl FileRepository,
    ctx: Ctx,
    request: FilesUploadRequest,
) -> Result<()> {
    let request_user_id: Id = request.user_id.into();
    if request_user_id != ctx.user_id() {
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
            uploader_id: ctx.user_id(),
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
    ctx: Ctx,
) -> Result<Json<Vec<FileMetadata>>> {
    debug!("Getting files metadata");

    let repo = DbFileRepository { db: state.db };
    let files = repo.find_synced_files(&ctx.user_id(), params.from).await?;

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

pub(super) async fn upload_file(
    State(state): State<AppState>,
    ctx: Ctx,
    Path(file_id): Path<Id>,
    mut multipart: Multipart,
) -> Result<()> {
    debug!(%file_id, "Uploading file");

    let repo = DbFileRepository { db: state.db };

    let file = repo.find(&file_id).await?.ok_or_else(|| {
        error!(%file_id, "Metadata not found for file upload");
        Error::FileNotFound
    })?;

    if file.uploader_id != ctx.user_id() {
        error!(%file_id, "Upload authorization mismatch");
        return Err(Error::Forbidden);
    }

    match file.state {
        FileState::New => {
            repo.update_state(&file_id, FileState::SyncInProgress)
                .await?;

            let mut original_uploaded = false;
            let mut thumbnails_uploaded = 0;

            while let Some(field) = multipart.next_field().await.map_err(|e| {
                error!(%file_id, error = %e, "Failed getting next multipart field");
                Error::FileUpload
            })? {
                let headers = field.headers().clone();
                let sha256 = headers.get("sha256_checksum").ok_or_else(|| {
                    error!(%file_id, "Missing sha256 checksum header");
                    Error::FileUpload
                })?;
                if sha256.is_empty() {
                    error!(%file_id, "Empty sha256 checksum header");
                    return Err(Error::FileUpload);
                }
                let sha256 = sha256.to_str().map_err(|e| {
                    error!(%file_id, error = %e, "Could not parse sha256 checksum header");
                    Error::FileUpload
                })?;

                match field.name() {
                    Some(ORIGINAL) => {
                        upload(
                            &file,
                            field,
                            ORIGINAL,
                            sha256,
                            &state.s3_client,
                            &state.config.storage.bucket_name,
                        )
                        .await?;
                        original_uploaded = true;
                    }
                    Some(name) if name.starts_with("thumbnail-") => {
                        let name = name.to_owned();
                        let thumb_name = name.strip_prefix("thumbnail-").unwrap();
                        let thumb: sdk::thumbnails::ThumbnailVariant =
                            thumb_name.parse().map_err(|_| {
                                error!(%file_id, name = thumb_name, "Invalid thumbnail variant");
                                Error::FileUpload
                            })?;
                        upload(
                            &file,
                            field,
                            &thumb.to_string(),
                            sha256,
                            &state.s3_client,
                            &state.config.storage.bucket_name,
                        )
                        .await?;
                        thumbnails_uploaded += 1;
                    }
                    _ => {
                        warn!(field_name = ?field.name(), "Ignoring unknown field");
                    }
                }
            }

            if !original_uploaded {
                error!(%file_id, "Original file was not uploaded");
                return Err(Error::FileUpload);
            }

            if thumbnails_uploaded != 2 {
                warn!(
                    %file_id,
                    count = thumbnails_uploaded,
                    "Expected 2 thumbnails",
                );
            }

            repo.update_state(&file_id, FileState::Synced).await?;
            Ok(())
        }
        _ => {
            error!(
                %file_id,
                state = ?file.state,
                "File should be in state New",
            );
            Err(Error::FileUpload)
        }
    }
}

const ORIGINAL: &str = "original";

async fn upload(
    file: &File,
    field: Field<'_>,
    field_name: &str,
    sha256: &str,
    client: &aws_sdk_s3::Client,
    bucket_name: &str,
) -> Result<()> {
    let file_id = file.id;
    debug!(%file_id, field_name, "Uploading file part");

    let content_type = field
        .content_type()
        .ok_or_else(|| {
            error!(%file_id, field_name, "Missing content type");
            Error::FileUpload
        })?
        .to_owned();

    let data = field.bytes().await.map_err(|e| {
        error!(%file_id, field_name, error = %e, "Could not read file bytes");
        Error::FileUpload
    })?;

    crypto::verify_data_hash(file.id.into(), sha256, &data)?;

    let file_key = format!("files/{}/{}/{}", file.owner_id, file.id, field_name);
    let result = client
        .put_object()
        .bucket(bucket_name)
        .key(&file_key)
        .content_type(content_type)
        .checksum_sha256(sha256)
        .body(ByteStream::from(data))
        .send()
        .await
        .map_err(|e| {
            error!(%file_id, field_name, error = %e, "Could not upload file to storage");
            Error::Storage
        })?;

    debug!(%file_id, field_name, ?result, "File part upload complete");
    Ok(())
}

#[derive(Debug, Deserialize)]
pub(super) struct FileDownloadParams {
    variant: Option<sdk::thumbnails::ThumbnailVariant>,
}

pub(super) async fn download_file(
    State(state): State<AppState>,
    ctx: Ctx,
    Path(file_id): Path<Id>,
    Query(params): Query<FileDownloadParams>,
) -> Result<(HeaderMap, Bytes)> {
    debug!(%file_id, variant = ?params.variant, "Downloading file");
    let repo = DbFileRepository { db: state.db };

    let file = repo.find(&file_id).await?.ok_or(Error::FileNotFound)?;

    if file.owner_id != ctx.user_id() {
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

        let ctx = Ctx::new(Id::new());

        // when
        let result = upload_files_metadata_internal(&mut repo, ctx, request).await;

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
        let ctx = Ctx::new(user_id.into());

        // when
        upload_files_metadata_internal(&mut repo, ctx, request)
            .await
            .unwrap();

        // then
        let file = &repo.files[0];
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
