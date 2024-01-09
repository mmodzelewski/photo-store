use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{request::Parts, Request},
    middleware::Next,
    response::Response,
};
use tracing::debug;

use super::error::Error as AuthError;
use crate::{
    ctx::Ctx,
    error::{Error, Result},
    AppState,
};

pub(crate) async fn require_auth(
    ctx: Result<Ctx>,
    request: Request<Body>,
    next: Next,
) -> Result<Response> {
    debug!("require_auth middleware");

    let ctx = ctx?;
    debug!("user identified: {:?}", ctx.user_id());

    Ok(next.run(request).await)
}

pub(crate) async fn ctx_resolver(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response> {
    debug!("ctx_resolver middleware");

    let auth_token = request
        .headers()
        .get("Authorization")
        .ok_or(AuthError::MissingAuthHeader)
        .and_then(|auth_token| {
            auth_token
                .to_str()
                .map_err(|_| AuthError::InvalidAuthHeader)
        });

    let db = state.db;
    let user_id = match auth_token {
        Ok(auth_token) => super::handlers::verify_token(&db, auth_token).await,
        Err(e) => Err(Error::AuthError(e)),
    };

    let result_ctx = user_id.map(|user_id| Ctx::new(user_id));

    request.extensions_mut().insert(result_ctx);

    debug!("ctx_resolver next");
    Ok(next.run(request).await)
}

#[async_trait::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Ctx {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _s: &S) -> Result<Self> {
        parts
            .extensions
            .get::<Result<Ctx>>()
            .ok_or::<Error>(AuthError::MissingAuthContext.into())?
            .clone()
    }
}
