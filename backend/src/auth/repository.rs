use uuid::Uuid;

use crate::database::DbPool;
use crate::error::Result;

pub(super) struct AuthRepository;

impl AuthRepository {
    pub async fn save_auth_token(db: &DbPool, user_id: &Uuid, auth_token: &str) -> Result<()> {
        let query = sqlx::query!(
            r#"INSERT INTO auth_token (
                user_id, token
            ) VALUES ($1, $2)"#,
            user_id,
            auth_token,
        );

        query.execute(db).await.map_err(|e| {
            crate::error::Error::DbError(format!("Could not save auth token {}", e))
        })?;

        Ok(())
    }
}
