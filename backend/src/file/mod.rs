mod handlers;
mod repository;
mod routes;

pub(crate) use routes::routes;
use time::OffsetDateTime;

#[derive(Debug, sqlx::Type, Clone)]
#[sqlx(type_name = "file_state")]
pub(crate) enum FileState {
    New,
    SyncInProgress,
    Synced,
}

#[derive(Debug, Clone)]
pub(crate) struct File {
    pub path: String,
    pub name: String,
    pub state: FileState,
    pub uuid: uuid::Uuid,
    pub created_at: OffsetDateTime,
    pub added_at: OffsetDateTime,
    pub sha256: String,
    pub owner_id: uuid::Uuid,
    pub uploader_id: uuid::Uuid,
    pub enc_key: String,
}
