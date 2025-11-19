use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("File upload error: {0}")]
    FileUpload(String),
    #[error("File download error: {0}")]
    FileDownload(String),
    #[error("File not found: {0}")]
    FileNotFound(uuid::Uuid),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Database error: {0}")]
    Database(String),
    #[error("Database migration error: {0}")]
    DbMigration(String),
    #[error("Auth error {0}")]
    Auth(#[from] crate::auth::error::Error),
    #[error("Password hashing error: {0}")]
    PasswordHashing(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Crypto error: {0}")]
    Crypto(#[from] crypto::error::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        println!("{:?}", self);

        if let Error::Auth(_) = self {
            return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
        }

        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    }
}
