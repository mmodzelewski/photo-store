use axum::{
    extract::multipart::MultipartError,
    response::{IntoResponse, Response},
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Multipart error: {0}")]
    MultipartError(#[from] MultipartError),
    #[error("Sql error: {0}")]
    SqlError(#[from] sqlx::Error),
    #[error("Database migration error: {0}")]
    DbMigrationError(#[from] sqlx::migrate::MigrateError),
    #[error("Auth error")]
    AuthError,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        println!("{:?}", self);

        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error",
        )
            .into_response()
    }
}

