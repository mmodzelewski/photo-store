use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use time::OffsetDateTime;
use tracing::debug;
use uuid::Uuid;

use super::repository::FileRepository;
use crate::{error::Result, file::FileState, AppState};

#[derive(Debug, serde::Deserialize)]
pub(super) struct NewFile {
    pub path: String,
    pub uuid: uuid::Uuid,
    #[serde(with = "time::serde::iso8601")]
    pub date: OffsetDateTime,
    pub sha256: String,
}

pub(super) async fn file_meta_upload(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(file): Json<NewFile>,
) -> Result<()> {
    debug!("{}", user_id);
    debug!("{:?}", &file);

    let db = state.db;
    let exists = FileRepository::exists(&db, &file.uuid).await?;

    if exists {
        debug!("File already exists");
        return Ok(());
    }

    debug!("Saving file");
    FileRepository::save(&db, &file).await?;

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

        while let Some(field) = multipart.next_field().await? {
            debug!("got field: {:?}", field.name());
            if Some("file") == field.name() {
                debug!("file content type: {:?}", field.content_type());
                debug!("file file name: {:?}", field.file_name());
                debug!("file headers: {:?}", field.headers());
                let _ = field.bytes().await?;
            }
        }
        Ok(())
    } else {
        debug!("File is not in New state");
        Ok(())
    };
}
