use time::OffsetDateTime;
use uuid::Uuid;

pub mod repository;

pub type FileId = Uuid;

#[derive(serde::Deserialize)]
pub struct NewFile {
    pub path: String,
    pub uuid: uuid::Uuid,
    #[serde(with = "time::serde::iso8601")]
    pub date: OffsetDateTime,
    pub sha256: String,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "file_state")]
pub enum FileState {
    New,
    SyncInProgress,
    Synced,
}
