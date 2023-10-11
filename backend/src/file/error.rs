pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQL query failed")]
    SqlError(#[from] sqlx::Error),
}
