use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use axum::body::Bytes;
use axum::extract::Query;
use axum::http::HeaderMap;
use axum::{
    extract::{multipart::Field, Multipart, Path, State},
    Json,
};
use serde::{de, Deserialize, Deserializer};
use time::OffsetDateTime;
use tracing::{debug, error, warn};
use uuid::Uuid;

use dtos::file::{FileMetadata, FilesUploadRequest};

use super::{repository::DbFileRepository, File};
use crate::config::StorageConfig;
use crate::file::repository::FileRepository;
use crate::{
    ctx::Ctx,
    error::{Error, Result},
    file::FileState,
    AppState,
};

pub(super) async fn upload_files_metadata(
    State(state): State<AppState>,
    ctx: Ctx,
    Json(request): Json<FilesUploadRequest>,
) -> Result<()> {
    debug!(
        "Uploading files metadata. Received {} items for user {}. Authenticated user {}",
        request.files.len(),
        request.user_id,
        ctx.user_id(),
    );

    let mut repo = DbFileRepository { db: state.db };
    upload_files_metadata_internal(&mut repo, ctx, request).await?;

    Ok(())
}

async fn upload_files_metadata_internal(
    repo: &mut impl FileRepository,
    ctx: Ctx,
    request: FilesUploadRequest,
) -> Result<()> {
    if request.user_id != ctx.user_id() {
        return Err(Error::FileUploadError(format!(
            "User {} is trying to upload files for user {}",
            ctx.user_id(),
            request.user_id
        )));
    }

    for item in request.files {
        let exists = repo.exists(&item.uuid).await?;

        if exists {
            warn!("File {:?} already exists, skipping upload", &item.uuid);
            continue;
        }

        let file = File {
            path: item.path.clone(),
            name: item.path.split('/').last().unwrap().to_string(),
            state: FileState::New,
            uuid: item.uuid,
            created_at: item.date,
            added_at: OffsetDateTime::now_utc(),
            sha256: item.sha256.clone(),
            owner_id: request.user_id,
            uploader_id: ctx.user_id(),
            enc_key: item.key.clone(),
        };

        debug!("Saving file {:?}", &file);
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
            let timestamp = value.parse::<i64>().map_err(|err| {
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
    debug!(
        "Getting files metadata for user {}. Params {:?}",
        ctx.user_id(),
        params
    );

    let repo = DbFileRepository { db: state.db };
    let files = repo.find_synced_files(&ctx.user_id(), params.from).await?;

    let metadata: Vec<FileMetadata> = files
        .into_iter()
        .map(|file| FileMetadata {
            path: file.path,
            uuid: file.uuid,
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
    Path(file_id): Path<Uuid>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<()> {
    debug!(
        "Uploading file: {:?}. Authenticated user {}",
        file_id,
        ctx.user_id()
    );

    let repo = DbFileRepository { db: state.db };

    let file = repo.find(&file_id).await?.ok_or_else(|| {
        Error::FileUploadError(format!("Metadata for file {:?} not found", &file_id))
    })?;
    debug!("Found file: {:?}", file);

    if file.uploader_id != ctx.user_id() {
        return Err(Error::FileUploadError(format!(
            "User {} is trying to upload file {} for user {}",
            ctx.user_id(),
            file_id,
            file.uploader_id
        )));
    }
    let sha256 = headers
        .get("sha256_checksum")
        .ok_or_else(|| Error::FileUploadError("Missing sha256 checksum header".to_string()))?;
    if sha256.is_empty() {
        return Err(Error::FileUploadError(
            "Empty sha256 checksum header".to_string(),
        ));
    }
    let sha256 = sha256.to_str().map_err(|e| {
        Error::FileUploadError(format!("Could not parse sha256 checksum header {}", e))
    })?;

    match file.state {
        FileState::New => {
            repo.update_state(&file_id, FileState::SyncInProgress)
                .await?;

            while let Some(field) = multipart.next_field().await.map_err(|e| {
                Error::FileUploadError(format!(
                    "Failed while getting next multipart field for file {}, error {}",
                    file_id, e
                ))
            })? {
                debug!("Got field: {:?}", &field);
                if Some("file") == field.name() {
                    upload(&file, field, sha256, &state.config.storage).await?;
                    repo.update_state(&file_id, FileState::Synced).await?;
                }
            }
            Ok(())
        }
        _ => {
            error!(
                "File {} should be in state New, but is in state {:?}",
                file_id, file.state
            );
            Err(Error::FileUploadError(format!(
                "File {} should be in state New, but is in state {:?}",
                file_id, file.state
            )))
        }
    }
}

async fn upload(file: &File, field: Field<'_>, sha256: &str, config: &StorageConfig) -> Result<()> {
    debug!("Uploading file {} data", file.uuid);

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region("auto")
        .endpoint_url(&config.url)
        .load()
        .await;
    // todo(mm): initialize client once
    let client = aws_sdk_s3::Client::new(&aws_config);

    let content_type = field
        .content_type()
        .ok_or(Error::FileUploadError(format!(
            "Missing content type for file {}",
            file.uuid
        )))?
        .to_owned();

    let data = field.bytes().await.map_err(|e| {
        error!("Could not read file {} bytes {}", file.uuid, e);
        Error::FileUploadError(format!("Could not read field bytes {}", e))
    })?;

    crypto::verify_data_hash(file.uuid, sha256, &data)?;

    let file_key = format!("files/{}/{}/original", file.owner_id, file.uuid);
    let result = client
        .put_object()
        .bucket(&config.bucket_name)
        .key(&file_key)
        .content_type(content_type)
        .checksum_sha256(sha256)
        .body(ByteStream::from(data))
        .send()
        .await
        .map_err(|e| {
            // improve error mapping
            let message = format!("Could not upload file {}, error: {}", file.uuid, e);
            error!(message);
            Error::FileUploadError(message)
        })?;

    debug!("File {} upload result: {:?}", file.uuid, result);
    Ok(())
}

pub(super) async fn download_file(
    State(state): State<AppState>,
    ctx: Ctx,
    Path(file_id): Path<Uuid>,
) -> Result<(HeaderMap, Bytes)> {
    debug!(
        "Downloading file: {:?}. Authenticated user {}",
        file_id,
        ctx.user_id()
    );
    let repo = DbFileRepository { db: state.db };

    let file = repo
        .find(&file_id)
        .await?
        .ok_or(Error::FileNotFound(file_id))?;

    if file.owner_id != ctx.user_id() {
        return Err(Error::Unauthorized);
    }

    let storage_config = &state.config.storage;
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region("auto")
        .endpoint_url(&storage_config.url)
        .load()
        .await;
    let client = aws_sdk_s3::Client::new(&aws_config);

    let file_key = format!("files/{}/{}/original", file.owner_id, file.uuid);
    debug!("Downloading file {} data", &file_key);
    let get_object = client
        .get_object()
        .bucket(&storage_config.bucket_name)
        .key(&file_key)
        .send()
        .await
        .map_err(|e| {
            let message = format!("Could not download file {}, error: {}", file_id, e);
            error!(message);
            Error::FileDownloadError(message)
        })?;

    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        get_object
            .content_type()
            .unwrap_or("application/octet-stream")
            .parse()
            .unwrap(),
    );
    headers.insert(
        http::header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", file.name)
            .parse()
            .unwrap(),
    );

    let file_bytes = get_object
        .body
        .collect()
        .await
        .map_err(|_| Error::FileDownloadError("Could not fetch".to_owned()))?
        .into_bytes();
    Ok((headers, file_bytes))
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
            user_id: Uuid::new_v4(),
            files: vec![],
        };

        let ctx = Ctx::new(Uuid::new_v4());

        // when
        let result = upload_files_metadata_internal(&mut repo, ctx, request).await;

        // then
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .starts_with("File upload error"));
    }

    #[tokio::test]
    async fn should_save_metadata_in_repository() {
        // given
        let mut repo = InMemoryFileRepository::new();
        let user_id = Uuid::new_v4();
        let request = FilesUploadRequest {
            user_id,
            files: vec![FileMetadata {
                path: "/home/pics/test.jpg".to_string(),
                uuid: Uuid::new_v4(),
                date: OffsetDateTime::now_utc(),
                sha256: "sha256".to_string(),
                key: "key".to_string(),
            }],
        };
        let metadata = request.files[0].clone();
        let ctx = Ctx::new(user_id);

        // when
        upload_files_metadata_internal(&mut repo, ctx, request)
            .await
            .unwrap();

        // then
        let file = &repo.files[0];
        assert_eq!(file.path, "/home/pics/test.jpg");
        assert_eq!(file.name, "test.jpg");
        assert!(matches!(file.state, FileState::New));
        assert_eq!(file.uuid, metadata.uuid);
        assert_eq!(file.created_at, metadata.date);
        assert_eq!(file.sha256, "sha256");
        assert_eq!(file.owner_id, user_id);
        assert_eq!(file.uploader_id, user_id);
        assert_eq!(file.enc_key, "key");
    }
}
