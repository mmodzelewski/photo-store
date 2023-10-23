#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Missing auth context")]
    MissingAuthContext,
    #[error("Missing auth header")]
    MissingAuthHeader,
    #[error("Invalid auth header")]
    InvalidAuthHeader,
    #[error("Invalid auth token")]
    InvalidAuthToken,
}
