use axum::{extract::DefaultBodyLimit, routing::get, Router};
use tower_http::limit::RequestBodyLimitLayer;

use crate::AppState;

use super::handlers;

pub(crate) fn routes(app_state: AppState) -> Router {
    Router::new()
        .route(
            "/files/metadata",
            get(handlers::get_files_metadata).post(handlers::upload_files_metadata),
        )
        .route(
            "/files/:file_id/data",
            get(handlers::download_file).post(handlers::upload_file),
        )
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024))
        .with_state(app_state)
}
