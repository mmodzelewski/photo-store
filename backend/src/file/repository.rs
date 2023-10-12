use uuid::Uuid;

use crate::database::DbPool;
use crate::error::Result;

use super::FileState;

pub(crate) struct FileRepository;

impl FileRepository {
    pub(crate) async fn exists(db: &DbPool, uuid: &Uuid) -> Result<bool> {
        let count = sqlx::query!("SELECT count(1) FROM file WHERE uuid = $1", uuid)
            .fetch_one(db)
            .await?;
        let res = count.count.map(|c| c > 0).unwrap_or(false);
        Ok(res)
    }

    pub(crate) async fn save(db: &DbPool, file: &super::NewFile) -> Result<()> {
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

        query.execute(db).await?;

        Ok(())
    }
}
