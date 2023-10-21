use axum::response::{IntoResponse, Response};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Missing auth context")]
    MissingAuthContext,
    #[error("Missing auth header")]
    MissingAuthHeader,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        println!("{:?}", self);

        (axum::http::StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
    }
}
