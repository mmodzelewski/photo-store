use std::fmt::Display;

use axum::{
    extract::multipart::MultipartError,
    response::{IntoResponse, Response},
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Generic,
    MultipartError(MultipartError),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Error::Generic
    }
}

impl From<MultipartError> for Error {
    fn from(value: MultipartError) -> Self {
        Error::MultipartError(value)
    }
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

impl std::error::Error for Error {}
