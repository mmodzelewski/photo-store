mod handlers;
mod repository;
mod routes;

pub(crate) use routes::routes;
use time::OffsetDateTime;

use crate::ulid::Id;

#[derive(Debug, sqlx::Type, Clone)]
#[sqlx(type_name = "file_state")]
pub(crate) enum FileState {
    New,
    SyncInProgress,
    Synced,
}

#[derive(Debug, Clone)]
pub(crate) struct File {
    pub id: Id,
    pub path: String,
    pub name: String,
    pub state: FileState,
    pub created_at: OffsetDateTime,
    #[allow(dead_code)]
    pub added_at: OffsetDateTime,
    pub sha256: String,
    pub owner_id: Id,
    pub uploader_id: Id,
    pub enc_key: String,
}

impl crypto::CryptoFileDesc for File {
    fn id(&self) -> ulid::Ulid {
        self.id.into()
    }

    fn sha256(&self) -> &str {
        &self.sha256
    }
}
