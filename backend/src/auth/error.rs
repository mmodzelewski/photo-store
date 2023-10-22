#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Missing auth context")]
    MissingAuthContext,
    #[error("Missing auth header")]
    MissingAuthHeader,
}
