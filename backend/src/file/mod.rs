use time::OffsetDateTime;

pub mod repository;
pub mod routes;

#[derive(serde::Deserialize)]
pub struct NewFile {
    pub path: String,
    pub uuid: uuid::Uuid,
    #[serde(with = "time::serde::iso8601")]
    pub date: OffsetDateTime,
    pub sha256: String,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "file_state")]
pub enum FileState {
    New,
    SyncInProgress,
    Synced,
}
