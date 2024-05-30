use uuid::Uuid;

use crate::database::DbPool;
use crate::error::Result;

use super::{File, FileState};

pub(super) struct FileRepository;

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

    pub(super) async fn find(db: &DbPool, uuid: &Uuid) -> Result<Option<File>> {
        let file = sqlx::query_as!(
            File,
            r#"SELECT
            path, name, state as "state: _", uuid,
            f.created_at, added_at, sha256,
            owner_id, uploader_id, enc_key
            FROM file f
            WHERE uuid = $1"#,
            uuid
        )
        .fetch_optional(db)
        .await
        .map_err(|e| crate::error::Error::DbError(format!("Could not get file {}", e)))?;

        Ok(file)
    }

    pub(super) async fn save(db: &DbPool, file: &File) -> Result<()> {
        let query = sqlx::query!(
            r#"INSERT INTO file (
                path, name, state, uuid, created_at, sha256, owner_id, uploader_id, enc_key 
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
            file.path,
            file.name,
            file.state as _,
            file.uuid,
            file.created_at,
            file.sha256,
            file.owner_id,
            file.uploader_id,
            file.enc_key,
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
            crate::error::Error::DbError(format!("Could not update file {} state {}", file_id, e))
        })?;

        Ok(())
    }
}
