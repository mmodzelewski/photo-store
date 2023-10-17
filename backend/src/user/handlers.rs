use axum::{extract::State, Json};

use crate::{error::Result, AppState};

use super::repository;

#[derive(serde::Deserialize)]
pub(super) struct UserRegister {
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub(super) struct UserRegisterResponse {
    pub user_id: String,
}

pub(super) async fn register(
    State(state): State<AppState>,
    Json(user): Json<UserRegister>,
) -> Result<Json<UserRegisterResponse>> {
    let db = state.db;

    repository::save(&db, &user).await?;

    return Ok(Json(UserRegisterResponse {
        user_id: "123".to_string(),
    }));
}
