use axum::{extract::DefaultBodyLimit, routing::post, Router};
use tower_http::limit::RequestBodyLimitLayer;

use crate::AppState;

use super::handlers;

pub(crate) fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/u/:id/files", post(handlers::file_meta_upload))
        .route(
            "/u/:user_id/files/:file_id/data",
            post(handlers::upload_file),
        )
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024))
        .with_state(app_state)
}
