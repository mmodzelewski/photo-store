use axum::routing::get;
use axum::{Router, routing::post};

use crate::AppState;

use super::handlers::{get_key, login, login_desktop, register, save_key};

pub(crate) fn routes(app_state: AppState) -> Router {
    let desktop = Router::new()
        .route("/desktop", get(login_desktop))
        .route_layer(axum::middleware::from_fn(super::middleware::require_auth))
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            super::middleware::session_resolver,
        ));

    Router::new()
        .route("/keys", get(get_key).post(save_key))
        .route_layer(axum::middleware::from_fn(super::middleware::require_auth))
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            super::middleware::session_resolver,
        ))
        .route("/login", post(login))
        .route("/register", post(register))
        .merge(desktop)
        .with_state(app_state)
}
