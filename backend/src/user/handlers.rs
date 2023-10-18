use axum::{extract::State, Json};
use uuid::Uuid;

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

    let user_id = Uuid::new_v4();
    repository::save(&db, &user_id, &user).await?;

    return Ok(Json(UserRegisterResponse {
        user_id: user_id.to_string(),
    }));
}
