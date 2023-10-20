use axum::{routing::post, Router};

use crate::AppState;

use super::handlers::login;

pub(crate) fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/login", post(login))
        .with_state(app_state)
}
