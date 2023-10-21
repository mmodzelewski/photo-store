use axum::{extract::State, Json};
use uuid::Uuid;

use crate::{error::Result, user::verify_user_password, AppState};

use super::repository::AuthRepository;

#[derive(serde::Deserialize)]
pub(super) struct UserLogin {
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub(super) struct AuthTokenResponse {
    pub auth_token: String,
}

pub(super) async fn login(
    State(state): State<AppState>,
    Json(user): Json<UserLogin>,
) -> Result<Json<AuthTokenResponse>> {
    let db = &state.db;
    let user_id = verify_user_password(db, &user.username, &user.password).await?;

    let auth_token = Uuid::new_v4().to_string();
    AuthRepository::save_auth_token(db, &user_id, &auth_token).await?;

    return Ok(Json(AuthTokenResponse { auth_token }));
}

pub(super) async fn verify_token(token: &str) -> Result<Uuid> {
    todo!()
}
