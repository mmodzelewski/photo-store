use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use time::OffsetDateTime;
use tracing::debug;
use uuid::Uuid;

use super::repository::FileRepository;
use crate::{
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

    let db = state.db;

    for file in metadata.items {
        let exists = FileRepository::exists(&db, &file.uuid).await?;

        if exists {
            debug!("File already exists");
            continue;
        }

        debug!("Saving file");
        FileRepository::save(&db, &file).await?;
    }

    Ok(())
}

pub(super) async fn upload_file(
    State(state): State<AppState>,
    Path((user_id, file_id)): Path<(Uuid, Uuid)>,
    mut multipart: Multipart,
) -> Result<()> {
    debug!("user_id: {:?}", user_id);
    debug!("file_id: {:?}", file_id);

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
                let _ = field.bytes().await.map_err(|e| {
                    Error::FileUploadError(format!("Could not read field bytes {}", e))
                })?;
            }
        }
        Ok(())
    } else {
        debug!("File is not in New state");
        Ok(())
    };
}
