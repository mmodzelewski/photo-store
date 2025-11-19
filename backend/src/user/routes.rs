use axum::{Router, routing::post};

use crate::AppState;

use super::handlers;

pub(crate) fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/user", post(handlers::register))
        .with_state(app_state)
}
