use axum::{extract::DefaultBodyLimit, routing::{post, get}, Router};
use tower_http::limit::RequestBodyLimitLayer;

use crate::AppState;

use super::handlers;

pub(crate) fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/files/metadata", post(handlers::upload_files_metadata))
        .route("/files/:file_id/data", post(handlers::upload_file))
        .route("/files/:file_id", get(handlers::get_file))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024))
        .with_state(app_state)
}
