use time::OffsetDateTime;

use crate::database::DbPool;
use crate::error::Result;
use crate::ulid::Id;

use super::{File, FileState};

pub(super) trait FileRepository {
    async fn exists(&self, id: &Id) -> Result<bool>;
    async fn find(&self, id: &Id) -> Result<Option<File>>;
    async fn find_synced_files(
        &self,
        user_id: &Id,
        from: Option<OffsetDateTime>,
    ) -> Result<Vec<File>>;
    async fn save(&mut self, file: &File) -> Result<()>;
    async fn update_state(&self, file_id: &Id, state: FileState) -> Result<()>;
}

pub(super) struct DbFileRepository {
    pub db: DbPool,
}

impl FileRepository for DbFileRepository {
    async fn exists(&self, id: &Id) -> Result<bool> {
        let count = sqlx::query!("SELECT count(1) FROM file WHERE id = $1", id as &Id)
            .fetch_one(&self.db)
            .await
            .map_err(|e| {
                crate::error::Error::Database(format!("Could not check if file exists {}", e))
            })?;
        let res = count.count.map(|c| c > 0).unwrap_or(false);
        Ok(res)
    }

    async fn find(&self, id: &Id) -> Result<Option<File>> {
        let file = sqlx::query_as!(
            File,
            r#"SELECT
            id as "id: Id", path, name, state as "state: _",
            f.created_at, added_at, sha256,
            owner_id as "owner_id: Id", uploader_id as "uploader_id: Id", enc_key
            FROM file f
            WHERE id = $1"#,
            id as &Id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| crate::error::Error::Database(format!("Could not get file {}", e)))?;

        Ok(file)
    }

    async fn find_synced_files(
        &self,
        user_id: &Id,
        from: Option<OffsetDateTime>,
    ) -> Result<Vec<File>> {
        let files = sqlx::query_as!(
            File,
            r#"SELECT
            id as "id: Id", path, name, state as "state: _",
            f.created_at, added_at, sha256,
            owner_id as "owner_id: Id", uploader_id as "uploader_id: Id", enc_key
            FROM file f
            WHERE owner_id = $1
            AND state = $2
            AND ($3::timestamptz IS NULL OR added_at >= $3)"#,
            user_id as &Id,
            FileState::Synced as _,
            from,
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            crate::error::Error::Database(format!("Could not get synced files for user {}", e))
        })?;

        Ok(files)
    }

    async fn save(&mut self, file: &File) -> Result<()> {
        let query = sqlx::query!(
            r#"INSERT INTO file (
                id, path, name, state, created_at, sha256, owner_id, uploader_id, enc_key
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
            &file.id as &Id,
            file.path,
            file.name,
            file.state as _,
            file.created_at,
            file.sha256,
            &file.owner_id as &Id,
            &file.uploader_id as &Id,
            file.enc_key,
        );

        query
            .execute(&self.db)
            .await
            .map_err(|e| crate::error::Error::Database(format!("Could not save file {}", e)))?;

        Ok(())
    }

    async fn update_state(&self, file_id: &Id, state: FileState) -> Result<()> {
        let query = sqlx::query!(
            "UPDATE file SET state = $1 WHERE id = $2",
            state as _,
            file_id as &Id
        );

        query.execute(&self.db).await.map_err(|e| {
            crate::error::Error::Database(format!("Could not update file {} state {}", file_id, e))
        })?;

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub struct InMemoryFileRepository {
        pub files: Vec<File>,
    }

    impl InMemoryFileRepository {
        pub fn new() -> Self {
            Self { files: Vec::new() }
        }
    }

    impl FileRepository for InMemoryFileRepository {
        async fn exists(&self, id: &Id) -> Result<bool> {
            let exists = self.files.iter().any(|f| f.id == *id);
            Ok(exists)
        }

        async fn find(&self, _id: &Id) -> Result<Option<File>> {
            todo!()
        }

        async fn find_synced_files(
            &self,
            user_id: &Id,
            from: Option<OffsetDateTime>,
        ) -> Result<Vec<File>> {
            Ok(self
                .files
                .iter()
                .filter(|f| f.owner_id == *user_id)
                .filter(|f| matches!(f.state, FileState::Synced))
                .filter(|f| from.is_none() || f.added_at >= from.unwrap())
                .cloned()
                .collect())
        }

        async fn save(&mut self, file: &File) -> Result<()> {
            self.files.push(file.clone());
            Ok(())
        }

        async fn update_state(&self, _file_id: &Id, _state: FileState) -> Result<()> {
            todo!()
        }
    }
}
