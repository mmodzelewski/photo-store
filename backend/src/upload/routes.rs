use axum::Router;
use axum::routing::{delete, get, post};

use crate::AppState;

use super::handlers;

pub(crate) fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/files/{file_id}/upload/init", post(handlers::init_upload))
        .route(
            "/files/{file_id}/upload/status",
            get(handlers::upload_status),
        )
        .route(
            "/files/{file_id}/upload/complete",
            post(handlers::complete_upload),
        )
        .route("/files/{file_id}/upload", delete(handlers::abort_upload))
        .with_state(app_state)
}
