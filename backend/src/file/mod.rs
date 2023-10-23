mod handlers;
mod repository;
mod routes;

pub(crate) use routes::routes;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "file_state")]
pub(crate) enum FileState {
    New,
    SyncInProgress,
    Synced,
}
