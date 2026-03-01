use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use time::OffsetDateTime;

use crate::database::DbPool;
use crate::entity::files;
use crate::entity::prelude::Files;
use crate::entity::sea_orm_active_enums::FileState as EntityFileState;
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
        let result = Files::find_by_id(uuid::Uuid::from(*id))
            .one(&self.db)
            .await
            .map_err(|e| {
                crate::error::Error::Database(format!("Could not check if file exists {}", e))
            })?;
        Ok(result.is_some())
    }

    async fn find(&self, id: &Id) -> Result<Option<File>> {
        let file = Files::find_by_id(uuid::Uuid::from(*id))
            .one(&self.db)
            .await
            .map_err(|e| crate::error::Error::Database(format!("Could not get file {}", e)))?
            .map(File::from);

        Ok(file)
    }

    async fn find_synced_files(
        &self,
        user_id: &Id,
        from: Option<OffsetDateTime>,
    ) -> Result<Vec<File>> {
        let mut query = Files::find()
            .filter(files::Column::OwnerId.eq(uuid::Uuid::from(*user_id)))
            .filter(files::Column::State.eq("Synced"));

        if let Some(from) = from {
            query = query.filter(files::Column::AddedAt.gte(from));
        }

        let files = query
            .all(&self.db)
            .await
            .map_err(|e| {
                crate::error::Error::Database(format!("Could not get synced files for user {}", e))
            })?
            .into_iter()
            .map(File::from)
            .collect();

        Ok(files)
    }

    async fn save(&mut self, file: &File) -> Result<()> {
        let entity_state: EntityFileState = file.state.clone().into();
        files::ActiveModel {
            id: Set(uuid::Uuid::from(file.id)),
            path: Set(file.path.clone()),
            name: Set(file.name.clone()),
            state: Set(entity_state),
            created_at: Set(file.created_at),
            added_at: Set(file.added_at),
            sha256: Set(file.sha256.clone()),
            owner_id: Set(uuid::Uuid::from(file.owner_id)),
            uploader_id: Set(uuid::Uuid::from(file.uploader_id)),
            enc_key: Set(file.enc_key.clone()),
        }
        .insert(&self.db)
        .await
        .map_err(|e| crate::error::Error::Database(format!("Could not save file {}", e)))?;

        Ok(())
    }

    async fn update_state(&self, file_id: &Id, state: FileState) -> Result<()> {
        let entity_state: EntityFileState = state.into();
        Files::update_many()
            .col_expr(files::Column::State, Expr::value(entity_state))
            .filter(files::Column::Id.eq(uuid::Uuid::from(*file_id)))
            .exec(&self.db)
            .await
            .map_err(|e| {
                crate::error::Error::Database(format!(
                    "Could not update file {} state {}",
                    file_id, e
                ))
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
