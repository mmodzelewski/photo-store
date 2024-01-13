use aes_gcm::aead::consts::U12;
use aes_gcm::Nonce;
use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit};
use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use axum::body::Bytes;
use axum::{
    extract::{multipart::Field, Multipart, Path, State},
    Json,
};
use base64ct::{Base64, Encoding};
use dtos::file::FilesUploadRequest;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::config::StorageConfig;
use crate::{
    ctx::Ctx,
    error::{Error, Result},
    file::FileState,
    AppState,
};

use super::{repository::FileRepository, File};

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

    if request.user_id != ctx.user_id() {
        return Err(Error::FileUploadError(format!(
            "User {} is trying to upload files for user {}",
            ctx.user_id(),
            request.user_id
        )));
    }

    let db = state.db;

    for item in request.files {
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
            owner_id: request.user_id.clone(),
            uploader_id: ctx.user_id(),
            key: None,
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
                    upload(&file, field, &state.config.storage).await?;
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

async fn upload(file: &File, field: Field<'_>, config: &StorageConfig) -> Result<()> {
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

    verify_data_hash(file, &data)?;

    let (encrypted_data, encrypted_data_hash) = encrypt_data(file, data)?;

    let file_key = format!("files/{}/{}/original", file.owner_id, file.uuid);
    let result = client
        .put_object()
        .bucket(&config.bucket_name)
        .key(&file_key)
        .content_type(content_type)
        .checksum_sha256(encrypted_data_hash)
        .body(ByteStream::from(encrypted_data))
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

fn encrypt_data(file: &File, data: Bytes) -> Result<(Vec<u8>, String)> {
    let encryption_key = decode_encryption_key(file)?;
    let aes256key = Key::<Aes256Gcm>::from_slice(&encryption_key);
    let cipher = Aes256Gcm::new(aes256key);
    let nonce = generate_nonce_from_uuid(file.uuid);

    let encrypted_data = cipher.encrypt(&nonce, data.as_ref()).unwrap();
    let data_hash = hash(&encrypted_data);
    Ok((encrypted_data, data_hash))
}

fn decode_encryption_key(file: &File) -> Result<Vec<u8>> {
    let encryption_key = file
        .key
        .as_ref()
        .ok_or(Error::FileUploadError(format!(
            "Missing encryption key for file {}",
            file.uuid
        )))
        .and_then(|k| {
            Base64::decode_vec(k).map_err(|e| {
                Error::FileUploadError(format!(
                    "Could not decode encryption key for file {}, error {}",
                    file.uuid, e
                ))
            })
        })?;
    Ok(encryption_key)
}

fn verify_data_hash(file: &File, data: &Bytes) -> Result<()> {
    let data_hash = hash(&data);
    if data_hash != file.sha256 {
        return Err(Error::FileUploadError(format!(
            "File {} hash mismatch, expected {}, got {}",
            file.uuid, file.sha256, data_hash
        )));
    }
    return Ok(());
}

fn hash(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    let encoded = Base64::encode_string(&hash);
    return encoded;
}

fn generate_nonce_from_uuid(uuid: Uuid) -> Nonce<U12> {
    let uuid_bytes = uuid.as_bytes();
    let hash = Sha256::digest(uuid_bytes);
    let nonce_bytes = &hash[0..12];
    Nonce::clone_from_slice(nonce_bytes)
}
