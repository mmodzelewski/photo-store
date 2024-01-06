use aes_gcm::{Aes256Gcm, Key, KeyInit};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{extract::State, Json};
use base64ct::{Base64, Encoding};
use tracing::debug;
use uuid::Uuid;

use crate::{
    database::DbPool,
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

    let enc_key: Key<Aes256Gcm> = Aes256Gcm::generate_key(OsRng);
    let encoded_key = Base64::encode_string(&enc_key);
    repository::save(&db, &user_id, &user, &encoded_key).await?;

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

pub(crate) async fn verify_user_password(
    db: &DbPool,
    username: &str,
    password: &str,
) -> Result<Uuid> {
    let user = repository::get_by_username(db, username).await?;
    verify_password(password, &user.password)?;
    Ok(user.uuid)
}

fn verify_password(password: &str, hash: &str) -> Result<()> {
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(hash).unwrap();

    argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|e| Error::PasswordHashingError(e.to_string()))?;
    Ok(())
}
