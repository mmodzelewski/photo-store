use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    routing::post,
    Json, Router,
};
use tower_http::limit::RequestBodyLimitLayer;
use tracing::debug;
use uuid::Uuid;

use crate::{error::Result, file::FileState, AppState};

use super::{repository::FileRepository, NewFile};

pub fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/u/:id/files", post(file_meta_upload))
        .route("/u/:user_id/files/:file_id/data", post(upload_file))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024))
        .with_state(app_state)
}

async fn file_meta_upload(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(file): Json<NewFile>,
) -> Result<()> {
    debug!("{}", user_id);
    debug!("{}", file.path);
    debug!("{}", file.uuid);
    debug!("{}", file.date);

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

pub async fn upload_file(
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
        FileRepository::update_state(&db, &file_id, FileState::SyncInProgress)
            .await?;

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
