mod handlers;
pub(crate) mod repository;
mod routes;

pub(crate) use routes::routes;
use time::OffsetDateTime;

use sdk::media::MediaType;

use crate::entity::sea_orm_active_enums::FileState as EntityFileState;
use crate::entity::sea_orm_active_enums::MediaType as EntityMediaType;
use crate::ulid::Id;

#[derive(Debug, Clone)]
pub(crate) enum FileState {
    New,
    SyncInProgress,
    Synced,
    Failed,
}

impl From<EntityFileState> for FileState {
    fn from(s: EntityFileState) -> Self {
        match s {
            EntityFileState::New => FileState::New,
            EntityFileState::SyncInProgress => FileState::SyncInProgress,
            EntityFileState::Synced => FileState::Synced,
            EntityFileState::Failed => FileState::Failed,
        }
    }
}

impl From<FileState> for EntityFileState {
    fn from(s: FileState) -> Self {
        match s {
            FileState::New => EntityFileState::New,
            FileState::SyncInProgress => EntityFileState::SyncInProgress,
            FileState::Synced => EntityFileState::Synced,
            FileState::Failed => EntityFileState::Failed,
        }
    }
}

impl From<EntityMediaType> for MediaType {
    fn from(m: EntityMediaType) -> Self {
        match m {
            EntityMediaType::Image => MediaType::Image,
            EntityMediaType::Video => MediaType::Video,
        }
    }
}

impl From<MediaType> for EntityMediaType {
    fn from(m: MediaType) -> Self {
        match m {
            MediaType::Image => EntityMediaType::Image,
            MediaType::Video => EntityMediaType::Video,
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
    pub media_type: MediaType,
    pub content_type: String,
    pub width: u32,
    pub height: u32,
    pub duration_ms: Option<u32>,
    pub segment_size: u32,
    pub plaintext_size: u64,
    pub nonce_salt: u32,
    pub enc_scheme: u8,
}

impl sdk::crypto::CryptoFileDesc for File {
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
            media_type: m.media_type.into(),
            content_type: m.content_type,
            width: m.width as u32,
            height: m.height as u32,
            duration_ms: m.duration_ms.map(|d| d as u32),
            segment_size: m.segment_size as u32,
            plaintext_size: m.plaintext_size as u64,
            nonce_salt: m.nonce_salt as u32,
            enc_scheme: m.enc_scheme as u8,
        }
    }
}
