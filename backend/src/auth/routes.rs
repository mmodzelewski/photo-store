use axum::routing::get;
use axum::{routing::post, Router};

use crate::auth::google;
use crate::AppState;

use super::handlers::login;

pub(crate) fn routes(app_state: AppState) -> Router {
    let google = Router::new()
        .route("/init", get(google::init_authentication))
        .route("/complete", get(google::complete_authentication));

    Router::new()
        .route("/login", post(login))
        .nest("/providers/google", google)
        .with_state(app_state)
}
