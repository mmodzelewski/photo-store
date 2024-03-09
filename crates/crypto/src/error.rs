pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Encryption error: {0}")]
    EncryptionError(String),
}
