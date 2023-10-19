use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use axum::{extract::State, Json};
use tracing::debug;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    AppState,
};

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
    debug!("Registering user: {}", user.username);
    let db = state.db;

    let user_id = Uuid::new_v4();

    let password_hash = hash_password(&user.password)?;

    let user = UserRegister {
        username: user.username,
        password: password_hash,
    };

    repository::save(&db, &user_id, &user).await?;

    let user_id = user_id.to_string();
    debug!("User registered: {}, {}", user.username, user_id);
    return Ok(Json(UserRegisterResponse { user_id }));
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::PasswordHashingError(e.to_string()))?;
    Ok(hash.to_string())
}
