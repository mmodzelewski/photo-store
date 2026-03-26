use axum::{Router, routing::get};

use crate::AppState;

use super::handlers;

pub(crate) fn routes(app_state: AppState) -> Router {
    Router::new()
        .route(
            "/files/metadata",
            get(handlers::get_files_metadata).post(handlers::upload_files_metadata),
        )
        .route("/files/{file_id}/data", get(handlers::download_file))
        .with_state(app_state)
}
