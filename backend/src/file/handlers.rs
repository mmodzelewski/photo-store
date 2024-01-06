use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use axum::{
    extract::{Multipart, multipart::Field, Path, State},
    Json,
};
use axum::body::Bytes;
use base64ct::{Base64, Encoding};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::{
    AppState,
    config::Config,
    ctx::Ctx,
    error::{Error, Result},
    file::FileState,
};

use super::{File, repository::FileRepository};

#[derive(Debug, serde::Deserialize)]
pub(super) struct FileMetadata {
    pub path: String,
    pub uuid: uuid::Uuid,
    #[serde(with = "time::serde::iso8601")]
    pub date: OffsetDateTime,
    pub sha256: String,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct FilesMetadata {
    pub user_id: uuid::Uuid,
    pub items: Vec<FileMetadata>,
}

pub(super) async fn upload_files_metadata(
    State(state): State<AppState>,
    ctx: Ctx,
    Json(metadata): Json<FilesMetadata>,
) -> Result<()> {
    debug!(
        "Uploading files metadata. Received {} items for user {}. Authenticated user {}",
        metadata.items.len(),
        metadata.user_id,
        ctx.user_id(),
    );

    if metadata.user_id != ctx.user_id() {
        return Err(Error::FileUploadError(format!(
            "User {} is trying to upload files for user {}",
            ctx.user_id(),
            metadata.user_id
        )));
    }

    let db = state.db;

    for item in metadata.items {
        let exists = FileRepository::exists(&db, &item.uuid).await?;

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
            sha256: item.sha256,
            owner_id: metadata.user_id.clone(),
            uploader_id: ctx.user_id(),
        };

        debug!("Saving file {:?}", &file);
        FileRepository::save(&db, &file).await?;
    }

    Ok(())
}

pub(super) async fn upload_file(
    State(state): State<AppState>,
    ctx: Ctx,
    Path(file_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<()> {
    debug!(
        "Uploading file: {:?}. Authenticated user {}",
        file_id,
        ctx.user_id()
    );

    let db = state.db;

    let file = FileRepository::find(&db, &file_id).await?.ok_or_else(|| {
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

    return match file.state {
        FileState::New => {
            FileRepository::update_state(&db, &file_id, FileState::SyncInProgress).await?;

            while let Some(field) = multipart.next_field().await.map_err(|e| {
                Error::FileUploadError(format!(
                    "Failed while getting next multipart field for file {}, error {}",
                    file_id, e
                ))
            })? {
                debug!("Got field: {:?}", &field);
                if Some("file") == field.name() {
                    upload(&file, field).await?;
                    FileRepository::update_state(&db, &file_id, FileState::Synced).await?;
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
    };
}

async fn upload(file: &File, field: Field<'_>) -> Result<()> {
    debug!("Uploading file {} data", file.uuid);

    // todo(mm): change config handling
    let local_config = Config::load().await.map_err(|e| {
        error!("Could not load config {}", e);
        Error::FileUploadError(format!("Could not load config {}", e))
    })?;

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region("auto")
        .endpoint_url(local_config.r2_url)
        .load()
        .await;
    // todo(mm): initialize client once
    let client = aws_sdk_s3::Client::new(&config);

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

    let hash = hash(&data)?;
    if hash != file.sha256 {
        return Err(Error::FileUploadError(format!(
            "File {} hash mismatch, expected {}, got {}",
            file.uuid, file.sha256, hash
        )));
    }

    let file_key = format!("files/{}/{}/original", file.owner_id, file.uuid);
    let result = client
        .put_object()
        .bucket(&local_config.bucket_name)
        .key(&file_key)
        .content_type(content_type)
        .checksum_sha256(&file.sha256)
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
    return Ok(());
}

fn hash(data: &Bytes) -> Result<String> {
    let hash = Sha256::digest(data);
    let encoded = Base64::encode_string(&hash);
    return Ok(encoded);
}
