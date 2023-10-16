mod handlers;
mod repository;
pub mod routes;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "file_state")]
pub(crate) enum FileState {
    New,
    SyncInProgress,
    Synced,
}
