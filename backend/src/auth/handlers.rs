use axum::{extract::State, Json};
use uuid::Uuid;

use crate::{error::Result, user::verify_user_password, AppState};

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
    Json(user): Json<UserLogin>,
) -> Result<Json<AuthTokenResponse>> {
    verify_user_password(&_state.db, &user.username, &user.password).await?;

    let auth_token = Uuid::new_v4();
    // todo: save auth token

    return Ok(Json(AuthTokenResponse { auth_token }));
}
