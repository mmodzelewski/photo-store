use uuid::Uuid;

use crate::auth::AuthorizationRequest;
use crate::database::DbPool;
use crate::error::{Error, Result};

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
            crate::error::Error::DbError(format!("Could not save auth token: {}", e))
        })?;

        Ok(())
    }

    pub async fn get_by_token(db: &DbPool, auth_token: &str) -> Result<Uuid> {
        let query = sqlx::query!(
            r#"SELECT user_id FROM auth_token WHERE token = $1"#,
            auth_token,
        );

        let row = query.fetch_one(db).await.map_err(|e| {
            crate::error::Error::DbError(format!("Could not get auth token: {}", e))
        })?;

        Ok(row.user_id)
    }

    pub async fn save_auth_request(db: &DbPool, auth_request: AuthorizationRequest) -> Result<()> {
        let query = sqlx::query!(
            r#"INSERT INTO authorization_requests (
                state, pkce
            ) VALUES ($1, $2)"#,
            auth_request.state,
            auth_request.pkce,
        );

        query.execute(db).await.map_err(|e| {
            crate::error::Error::DbError(format!("Could not save auth request: {}", e))
        })?;

        Ok(())
    }

    pub async fn get_auth_request_by_state(
        db: &DbPool,
        state: &str,
    ) -> Result<AuthorizationRequest> {
        let query = sqlx::query_as!(
            AuthorizationRequest,
            r#"SELECT state, pkce FROM authorization_requests WHERE state = $1"#,
            state,
        );

        let auth_request = query.fetch_one(db).await.map_err(|e| {
            crate::error::Error::DbError(format!("Could not get auth request: {}", e))
        })?;

        Ok(auth_request)
    }

    pub(crate) async fn save_keys(
        db: &sqlx::Pool<sqlx::Postgres>,
        user_id: &Uuid,
        private_key: &str,
        public_key: &str,
    ) -> Result<()> {
        let query = sqlx::query!(
            r#"INSERT INTO user_keys (
                user_id, private_key, public_key
            ) VALUES ($1, $2, $3)"#,
            user_id,
            private_key,
            public_key,
        );

        query
            .execute(db)
            .await
            .map_err(|e| Error::DbError(format!("Could not save user keys: {}", e)))?;

        Ok(())
    }

    pub(crate) async fn get_private_key(
        db: &sqlx::Pool<sqlx::Postgres>,
        user_id: &Uuid,
    ) -> Result<Option<String>> {
        let query = sqlx::query!(
            r#"SELECT private_key FROM user_keys where user_id = $1"#,
            user_id
        );
        let result = query
            .fetch_optional(db)
            .await
            .map_err(|e| Error::DbError(format!("Could not get private_key {}", e)))?
            .map(|row| row.private_key);

        Ok(result)
    }
}
