use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, JoinType, PaginatorTrait, QueryFilter, QuerySelect,
    RelationTrait, Set,
};
use time::OffsetDateTime;
use tracing::error;

use crate::database::DbPool;
use crate::entity::prelude::UploadSessions;
use crate::entity::upload_sessions;
use crate::error::Result;
use crate::ulid::Id;

use super::UploadSession;

pub(crate) trait UploadRepository {
    async fn create_session(&mut self, session: &UploadSession) -> Result<()>;
    async fn find_session(&self, file_id: &Id) -> Result<Option<UploadSession>>;
    async fn delete_session(&self, file_id: &Id) -> Result<()>;
    async fn find_expired_sessions(&self, now: OffsetDateTime) -> Result<Vec<UploadSession>>;
    async fn count_user_sessions(&self, user_id: &Id) -> Result<i64>;
}

pub(crate) struct DbUploadRepository {
    pub db: DbPool,
}

impl UploadRepository for DbUploadRepository {
    async fn create_session(&mut self, session: &UploadSession) -> Result<()> {
        upload_sessions::ActiveModel {
            file_id: Set(uuid::Uuid::from(session.file_id)),
            upload_id: Set(session.upload_id.clone()),
            total_size: Set(session.total_size),
            chunk_size: Set(session.chunk_size),
            total_chunks: Set(session.total_chunks),
            created_at: Set(session.created_at),
            expires_at: Set(session.expires_at),
        }
        .insert(&self.db)
        .await
        .map_err(|e| {
            error!(error = %e, "Could not create upload session");
            crate::error::Error::Database
        })?;

        Ok(())
    }

    async fn find_session(&self, file_id: &Id) -> Result<Option<UploadSession>> {
        let session = UploadSessions::find_by_id(uuid::Uuid::from(*file_id))
            .one(&self.db)
            .await
            .map_err(|e| {
                error!(error = %e, "Could not find upload session");
                crate::error::Error::Database
            })?
            .map(UploadSession::from);

        Ok(session)
    }

    async fn delete_session(&self, file_id: &Id) -> Result<()> {
        UploadSessions::delete_by_id(uuid::Uuid::from(*file_id))
            .exec(&self.db)
            .await
            .map_err(|e| {
                error!(error = %e, "Could not delete upload session");
                crate::error::Error::Database
            })?;

        Ok(())
    }

    async fn find_expired_sessions(&self, now: OffsetDateTime) -> Result<Vec<UploadSession>> {
        let sessions = UploadSessions::find()
            .filter(upload_sessions::Column::ExpiresAt.lt(now))
            .all(&self.db)
            .await
            .map_err(|e| {
                error!(error = %e, "Could not find expired upload sessions");
                crate::error::Error::Database
            })?
            .into_iter()
            .map(UploadSession::from)
            .collect();

        Ok(sessions)
    }

    async fn count_user_sessions(&self, user_id: &Id) -> Result<i64> {
        use crate::entity::files;

        let count = UploadSessions::find()
            .join(JoinType::InnerJoin, upload_sessions::Relation::Files.def())
            .filter(files::Column::UploaderId.eq(uuid::Uuid::from(*user_id)))
            .count(&self.db)
            .await
            .map_err(|e| {
                error!(error = %e, "Could not count user upload sessions");
                crate::error::Error::Database
            })?;

        Ok(count as i64)
    }
}
