use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State};
use tracing::debug;
use uuid::Uuid;

use crate::{
    AppState,
    database::DbPool,
    error::{Error, Result},
};

use super::repository::{self, AccountProvider};

#[derive(serde::Deserialize)]
pub(super) struct UserRegister {
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub(super) struct UserRegisterResponse {
    pub user_id: String,
}

#[derive(Debug)]
pub(crate) struct UserId(pub Uuid);

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

    repository::save_user_with_credentials(&db, &user_id, &user).await?;

    let user_id = user_id.to_string();
    debug!("User registered: {}, {}", user.username, user_id);
    Ok(Json(UserRegisterResponse { user_id }))
}

pub(crate) async fn register_or_get_with_external_provider(
    db: &DbPool,
    account_id: &str,
    provider: &AccountProvider,
) -> Result<UserId> {
    debug!("Registering or getting user with external provider");

    let user_id = repository::find_by_provider(db, account_id, provider).await?;
    if let Some(user_id) = user_id {
        debug!("User found with external provider: {}", &user_id.0);
        return Ok(user_id);
    }

    let user_id = Uuid::new_v4();
    repository::save_user_with_external_provider(db, &user_id, account_id, provider).await?;
    debug!("User registered with external provider: {}", &user_id);

    Ok(UserId(user_id))
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::PasswordHashing(e.to_string()))?;
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
        .map_err(|e| Error::PasswordHashing(e.to_string()))?;
    Ok(())
}
