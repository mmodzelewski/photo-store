use uuid::Uuid;

use crate::database::DbPool;
use crate::error::Result;

use super::FileState;

pub(super) struct FileRepository;

struct StateRow {
    state: FileState,
}

impl FileRepository {
    pub(super) async fn exists(db: &DbPool, uuid: &Uuid) -> Result<bool> {
        let count = sqlx::query!("SELECT count(1) FROM file WHERE uuid = $1", uuid)
            .fetch_one(db)
            .await
            .map_err(|e| {
                crate::error::Error::DbError(format!("Could not check if file exists {}", e))
            })?;
        let res = count.count.map(|c| c > 0).unwrap_or(false);
        Ok(res)
    }

    pub(super) async fn get_state(db: &DbPool, uuid: &Uuid) -> Result<Option<FileState>> {
        let state = sqlx::query_as!(
            StateRow,
            r#"SELECT state as "state: _" FROM file WHERE uuid = $1"#,
            uuid
        )
        .fetch_optional(db)
        .await
        .map_err(|e| crate::error::Error::DbError(format!("Could not get file state {}", e)))?;

        Ok(state.map(|s| s.state))
    }

    pub(super) async fn save(db: &DbPool, file: &super::handlers::FileMetadata) -> Result<()> {
        let query = sqlx::query!(
            r#"INSERT INTO file (
                path, name, state, uuid, created_at, sha256
            ) VALUES ($1, $2, $3, $4, $5, $6)"#,
            file.path,
            file.path,
            FileState::New as _,
            file.uuid,
            file.date,
            file.sha256
        );

        query
            .execute(db)
            .await
            .map_err(|e| crate::error::Error::DbError(format!("Could not save file {}", e)))?;

        Ok(())
    }

    pub(super) async fn update_state(db: &DbPool, file_id: &Uuid, state: FileState) -> Result<()> {
        let query = sqlx::query!(
            "UPDATE file SET state = $1 WHERE uuid = $2",
            state as _,
            file_id
        );

        query.execute(db).await.map_err(|e| {
            crate::error::Error::DbError(format!("Could not update file state {}", e))
        })?;

        Ok(())
    }
}
