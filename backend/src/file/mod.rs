mod handlers;
mod repository;
mod routes;

pub(crate) use routes::routes;
use time::OffsetDateTime;

use crate::entity::sea_orm_active_enums::FileState as EntityFileState;
use crate::ulid::Id;

#[derive(Debug, Clone)]
pub(crate) enum FileState {
    New,
    SyncInProgress,
    Synced,
}

impl From<EntityFileState> for FileState {
    fn from(s: EntityFileState) -> Self {
        match s {
            EntityFileState::New => FileState::New,
            EntityFileState::SyncInProgress => FileState::SyncInProgress,
            EntityFileState::Synced => FileState::Synced,
        }
    }
}

impl From<FileState> for EntityFileState {
    fn from(s: FileState) -> Self {
        match s {
            FileState::New => EntityFileState::New,
            FileState::SyncInProgress => EntityFileState::SyncInProgress,
            FileState::Synced => EntityFileState::Synced,
        }
    }
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

impl From<crate::entity::files::Model> for File {
    fn from(m: crate::entity::files::Model) -> Self {
        File {
            id: Id::from(m.id),
            path: m.path,
            name: m.name,
            state: m.state.into(),
            created_at: m.created_at,
            added_at: m.added_at,
            sha256: m.sha256,
            owner_id: Id::from(m.owner_id),
            uploader_id: Id::from(m.uploader_id),
            enc_key: m.enc_key,
        }
    }
}
