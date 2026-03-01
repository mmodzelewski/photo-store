use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("File upload error")]
    FileUpload,
    #[error("File download error")]
    FileDownload,
    #[error("Storage error")]
    Storage,
    #[error("File not found")]
    FileNotFound,
    #[error("Forbidden")]
    Forbidden,
    #[error("Database error")]
    Database,
    #[error("Database migration error")]
    DbMigration,
    #[error("Auth error: {0}")]
    Auth(#[from] crate::auth::error::Error),
    #[error("Password hashing error")]
    PasswordHashing,
    #[error("Configuration error")]
    Configuration,
    #[error("Crypto error")]
    Crypto(#[from] crypto::error::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::Auth(_) => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            Error::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            Error::FileNotFound => (StatusCode::NOT_FOUND, "Not found"),
            Error::FileUpload | Error::FileDownload => (StatusCode::BAD_REQUEST, "Bad request"),
            Error::Storage
            | Error::Database
            | Error::DbMigration
            | Error::PasswordHashing
            | Error::Configuration
            | Error::Crypto(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };

        tracing::error!(status = status.as_u16(), error = %self, "Request failed");

        (status, message).into_response()
    }
}
