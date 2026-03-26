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
    #[error("Upload session not found")]
    UploadNotFound,
    #[error("Upload conflict")]
    UploadConflict,
    #[error("Upload incomplete")]
    UploadIncomplete,
    #[error("Upload expired")]
    UploadExpired,
    #[error("Rate limit exceeded")]
    TooManyRequests,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::Auth(_) => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            Error::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            Error::FileNotFound | Error::UploadNotFound => (StatusCode::NOT_FOUND, "Not found"),
            Error::FileUpload | Error::FileDownload | Error::UploadIncomplete => {
                (StatusCode::BAD_REQUEST, "Bad request")
            }
            Error::UploadConflict => (StatusCode::CONFLICT, "Conflict"),
            Error::UploadExpired => (StatusCode::GONE, "Gone"),
            Error::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "Too many requests"),
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
