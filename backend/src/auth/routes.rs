use axum::{extract::State, routing::post, Json, Router};
use uuid::Uuid;

use crate::{error::Result, AppState};

pub(crate) fn routes(app_state: AppState) -> Router {
    Router::new()
        .route("/login", post(login))
        .with_state(app_state)
}

#[derive(serde::Deserialize)]
pub(super) struct UserLogin {
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub(super) struct AuthTokenResponse {
    pub auth_token: Uuid,
}

pub(super) async fn login(
    State(_state): State<AppState>,
    Json(_user): Json<UserLogin>,
) -> Result<Json<AuthTokenResponse>> {
    // todo: check user
    // todo: save auth token

    return Ok(Json(AuthTokenResponse {
        auth_token: Uuid::new_v4(),
    }));
}
