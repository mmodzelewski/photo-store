use axum::{
    extract::FromRequestParts,
    http::{request::Parts, Request},
    middleware::Next,
    response::Response,
};
use tracing::debug;
use uuid::Uuid;

use crate::{
    ctx::Ctx,
    error::{Error, Result},
};

pub(crate) async fn require_auth<B>(
    ctx: Result<Ctx>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    debug!("require_auth");

    let ctx = ctx?;
    debug!("user_id: {:?}", ctx.user_id());

    Ok(next.run(request).await)
}

#[async_trait::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Ctx {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
        let _auth_token = parts.headers.get("Authorization").ok_or(Error::AuthError)?;
        // todo: validate auth token

        return Ok(Self::new(Uuid::new_v4()));
    }
}
