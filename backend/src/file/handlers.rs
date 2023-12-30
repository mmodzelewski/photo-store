use aws_sdk_s3::primitives::ByteStream;
use axum::{
    extract::{multipart::Field, Multipart, Path, State},
    Json,
};
use time::OffsetDateTime;
use tracing::{debug, error};
use uuid::Uuid;

use super::repository::FileRepository;
use crate::{
    config::Config,
    ctx::Ctx,
    error::{Error, Result},
    file::FileState,
    AppState,
};

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
    let user_id = &metadata.user_id;
    debug!("Upload for user: {}", user_id);
    debug!("Logged in user: {}", ctx.user_id());
    debug!("{:?}", &metadata);
    // save logged in user with the file

    let db = state.db;

    for file in metadata.items {
        let exists = FileRepository::exists(&db, &file.uuid).await?;

        if exists {
            debug!("File {:?} already exists", &file.uuid);
            continue;
        }

        debug!("Saving file {:?}", &file.uuid);
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
    debug!("file_id: {:?}", file_id);
    debug!("user id: {:?}", ctx.user_id());
    // verify if the logged in user mathes the user saved with metadata

    let db = state.db;

    let state = FileRepository::get_state(&db, &file_id).await?;
    debug!("state: {:?}", state);

    return if let Some(FileState::New) = state {
        FileRepository::update_state(&db, &file_id, FileState::SyncInProgress).await?;

        while let Some(field) = multipart.next_field().await.map_err(|e| {
            Error::FileUploadError(format!("Failed while getting next multipart field {}", e))
        })? {
            debug!("got field: {:?}", field.name());
            if Some("file") == field.name() {
                debug!("file content type: {:?}", field.content_type());
                debug!("file file name: {:?}", field.file_name());
                debug!("file headers: {:?}", field.headers());
                //let _ = field.bytes().await.map_err(|e| {
                //    Error::FileUploadError(format!("Could not read field bytes {}", e))
                //})?;
                upload(file_id, field).await;
                FileRepository::update_state(&db, &file_id, FileState::Synced).await?;
            }
        }
        Ok(())
    } else {
        debug!("File {:?} is not in New state", &file_id);
        Ok(())
    };
}

async fn upload(file_id: Uuid, field: Field<'_>) {
    debug!("uploading file: {:?}", file_id);
    let local_config = Config::load().await;
    if local_config.is_err() {
        error!("{:?}", local_config);
        return;
    }
    let local_config = local_config.unwrap();
    let key = format!("files/{}", file_id);

    let config = aws_config::from_env()
        .region("auto")
        .endpoint_url(local_config.r2_url)
        .load()
        .await;
    let client = aws_sdk_s3::Client::new(&config);

    let file_name = field.file_name().unwrap().to_string();
    let content_type = field.content_type().unwrap().to_string();
    let data = field.bytes().await.unwrap();

    debug!(
        "Length of `file` (`{}`: `{}`) is {} bytes",
        file_name,
        content_type,
        data.len(),
    );

    let stream = ByteStream::from(data);

    let result = client
        .put_object()
        .bucket(local_config.bucket_name)
        .key(key)
        .content_type("image/jpeg")
        .body(stream)
        .send()
        .await;

    println!("{:?}", result);
}

