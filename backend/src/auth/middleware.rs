use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{Request, request::Parts},
    middleware::Next,
    response::Response,
};
use tracing::debug;

use crate::{
    AppState,
    error::{Error, Result},
    session::Session,
};

use super::error::Error as AuthError;

pub(crate) async fn require_auth(
    session: Result<Session>,
    request: Request<Body>,
    next: Next,
) -> Result<Response> {
    debug!("require_auth middleware");

    let session = session?;
    debug!("user identified: {:?}", session.user_id());

    Ok(next.run(request).await)
}

pub(crate) async fn session_resolver(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response> {
    debug!("session_resolver middleware");

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
        Err(e) => Err(Error::Auth(e)),
    };

    let result_session = user_id.map(Session::new);

    request.extensions_mut().insert(result_session);

    debug!("session_resolver next");
    Ok(next.run(request).await)
}

impl<S: Send + Sync> FromRequestParts<S> for Session {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _s: &S) -> Result<Self> {
        parts
            .extensions
            .get::<Result<Session>>()
            .ok_or::<Error>(AuthError::MissingAuthContext.into())?
            .clone()
    }
}
