use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("File upload error: {0}")]
    FileUploadError(String),
    #[error("File download error: {0}")]
    FileDownloadError(String),
    #[error("File not found: {0}")]
    FileNotFound(uuid::Uuid),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Database error: {0}")]
    DbError(String),
    #[error("Database migration error: {0}")]
    DbMigrationError(String),
    #[error("Auth error {0}")]
    AuthError(#[from] crate::auth::error::Error),
    #[error("Password hashing error: {0}")]
    PasswordHashingError(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Crypto error: {0}")]
    CryptoError(#[from] crypto::error::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        println!("{:?}", self);

        if let Error::AuthError(_) = self {
            return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
        }

        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    }
}
