use axum::{
    extract::{FromRequestParts, State},
    http::{request::Parts, Request},
    middleware::Next,
    response::Response,
};
use tracing::debug;
use uuid::Uuid;

use super::error::Error;
use super::error::Result;
use crate::{ctx::Ctx, AppState};

pub(crate) async fn require_auth<B>(
    ctx: Result<Ctx>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    debug!("require_auth middleware");

    let ctx = ctx?;
    debug!("user identified: {:?}", ctx.user_id());

    Ok(next.run(request).await)
}

pub(crate) async fn ctx_resolver<B>(
    State(_state): State<AppState>,
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    debug!("ctx_resolver middleware");

    let _auth_token = request
        .headers()
        .get("Authorization")
        .ok_or(Error::MissingAuthHeader)?;

    // super::handlers::verify_token("").await?;
    // todo: validate auth token
    // todo: get user id

    let result_ctx = Ok::<_, Error>(Ctx::new(Uuid::new_v4()));

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
            .ok_or(Error::MissingAuthContext)?
            .clone()
    }
}
