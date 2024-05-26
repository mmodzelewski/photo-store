use axum::routing::get;
use axum::{routing::post, Router};

use crate::auth::google;
use crate::AppState;

use super::handlers::{login, login_desktop};

pub(crate) fn routes(app_state: AppState) -> Router {
    let google = Router::new()
        .route("/init", get(google::init_authentication))
        .route("/complete", get(google::complete_authentication));

    let desktop = Router::new()
        .route("/desktop", get(login_desktop))
        .route_layer(axum::middleware::from_fn(super::middleware::require_auth))
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            super::middleware::ctx_resolver,
        ));

    Router::new()
        .route("/login", post(login))
        .merge(desktop)
        .nest("/providers/google", google)
        .with_state(app_state)
}
