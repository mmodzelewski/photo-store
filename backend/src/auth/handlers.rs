use crate::{AppState, ctx::Ctx, database::DbPool, error::Result, ulid::Id};
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Json,
    extract::{Query, State},
};
use dtos::auth::{LoginRequest, LoginResponse, PrivateKeyResponse, SaveRsaKeysRequest};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use uuid::Uuid;

use super::{error::Error, repository::AuthRepository};

#[derive(serde::Deserialize)]
pub(super) struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub(super) struct RegisterResponse {
    pub user_id: String,
}

pub(super) async fn register(
    State(state): State<AppState>,
    Json(user): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>> {
    if !state.config.registration_enabled {
        return Err(Error::RegistrationDisabled.into());
    }

    debug!("Registering user: {}", user.username);
    let db = &state.db;

    let user_id = Id::new();
    let password_hash = hash_password(&user.password)?;

    AuthRepository::save_user_with_credentials(db, &user_id, &user.username, &password_hash)
        .await?;

    let user_id = user_id.to_string();
    debug!("User registered: {}", user.username);
    Ok(Json(RegisterResponse { user_id }))
}

pub(super) async fn login(
    State(state): State<AppState>,
    Json(user): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    let db = &state.db;
    let user_id = verify_user_password(db, &user.username, &user.password).await?;

    let auth_token = Uuid::new_v4().to_string();
    AuthRepository::save_auth_token(db, &user_id, &auth_token).await?;

    Ok(Json(LoginResponse {
        user_id: user_id.into(),
        auth_token,
    }))
}

pub(super) async fn save_key(
    State(state): State<AppState>,
    ctx: Ctx,
    Json(keys): Json<SaveRsaKeysRequest>,
) -> Result<()> {
    debug!("Saving keys for user");
    let db = &state.db;

    AuthRepository::save_keys(db, &ctx.user_id(), &keys.private_key, &keys.public_key).await?;
    Ok(())
}

pub(super) async fn get_key(
    State(state): State<AppState>,
    ctx: Ctx,
) -> Result<Json<PrivateKeyResponse>> {
    debug!("Getting keys for user");
    let db = &state.db;

    let pk = AuthRepository::get_private_key(db, &ctx.user_id()).await?;
    Ok(Json(PrivateKeyResponse { value: pk }))
}

#[derive(Serialize, Deserialize)]
pub struct RedirectUri {
    redirect_uri: String,
}

pub(super) async fn login_desktop(
    State(state): State<AppState>,
    redirect_uri: Query<RedirectUri>,
    ctx: Ctx,
) -> Result<Json<RedirectUri>> {
    let db = &state.db;

    let auth_token = Uuid::new_v4().to_string();
    AuthRepository::save_auth_token(db, &ctx.user_id(), &auth_token).await?;

    let redirect_uri = format!(
        "{}?auth_token={}&user_id={}",
        redirect_uri.redirect_uri,
        auth_token,
        ctx.user_id(),
    );
    Ok(Json(RedirectUri { redirect_uri }))
}

pub(super) async fn verify_token(db: &DbPool, token: &str) -> Result<Id> {
    AuthRepository::get_by_token(db, token)
        .await
        .map_err(|_| Error::InvalidAuthToken.into())
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| {
            error!(error = %e, "Password hashing failed");
            crate::error::Error::PasswordHashing
        })?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> std::result::Result<(), Error> {
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(hash).map_err(|_| Error::InvalidCredentials)?;

    argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| Error::InvalidCredentials)?;
    Ok(())
}

async fn verify_user_password(db: &DbPool, username: &str, password: &str) -> Result<Id> {
    let user = AuthRepository::get_by_username(db, username).await;

    let user = match user {
        Ok(user) => user,
        Err(_) => {
            debug!("Login attempt for non-existent user");
            return Err(Error::InvalidCredentials.into());
        }
    };

    verify_password(password, &user.password).map_err(|_| {
        debug!("Login attempt with invalid password");
        Error::InvalidCredentials
    })?;

    Ok(user.id)
}
