use axum::{
    extract::{Query, State},
    Json,
};
use dtos::auth::{LoginRequest, LoginResponse, PrivateKeyResponse, SaveRsaKeysRequest};
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use crate::{ctx::Ctx, database::DbPool, error::Result, user::verify_user_password, AppState};

use super::{error::Error, repository::AuthRepository};

pub(super) async fn login(
    State(state): State<AppState>,
    Json(user): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    let db = &state.db;
    let user_id = verify_user_password(db, &user.username, &user.password).await?;

    let auth_token = Uuid::new_v4().to_string();
    AuthRepository::save_auth_token(db, &user_id, &auth_token).await?;

    Ok(Json(LoginResponse {
        user_id,
        auth_token,
    }))
}

pub(super) async fn save_key(
    State(state): State<AppState>,
    ctx: Ctx,
    Json(keys): Json<SaveRsaKeysRequest>,
) -> Result<()> {
    debug!("Saving keys for user: {}", ctx.user_id());
    let db = &state.db;

    AuthRepository::save_keys(db, &ctx.user_id(), &keys.private_key, &keys.public_key).await?;
    Ok(())
}

pub(super) async fn get_key(
    State(state): State<AppState>,
    ctx: Ctx,
) -> Result<Json<PrivateKeyResponse>> {
    debug!("Getting keys for user: {}", ctx.user_id());
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

pub(super) async fn verify_token(db: &DbPool, token: &str) -> Result<Uuid> {
    AuthRepository::get_by_token(db, token)
        .await
        .map_err(|_| Error::InvalidAuthToken.into())
}
