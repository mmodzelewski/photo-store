use axum::{extract::State, Json};
use uuid::Uuid;

use crate::{database::DbPool, error::Result, user::verify_user_password, AppState};

use super::{error::Error, repository::AuthRepository};

#[derive(serde::Deserialize)]
pub(super) struct UserLogin {
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub(super) struct LoginResponse {
    pub user_id: Uuid,
    pub auth_token: String,
}

pub(super) async fn login(
    State(state): State<AppState>,
    Json(user): Json<UserLogin>,
) -> Result<Json<LoginResponse>> {
    let db = &state.db;
    let user_id = verify_user_password(db, &user.username, &user.password).await?;

    let auth_token = Uuid::new_v4().to_string();
    AuthRepository::save_auth_token(db, &user_id, &auth_token).await?;

    return Ok(Json(LoginResponse {
        user_id,
        auth_token,
    }));
}

pub(super) async fn verify_token(db: &DbPool, token: &str) -> Result<Uuid> {
    AuthRepository::get_by_token(db, token)
        .await
        .map_err(|_| Error::InvalidAuthToken.into())
}
