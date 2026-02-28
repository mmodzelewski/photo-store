use uuid::Uuid;

use crate::database::DbPool;
use crate::error::{Error, Result};

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "provider")]
pub(super) enum AccountProvider {
    Credentials,
}

pub(super) struct User {
    pub uuid: Uuid,
    pub password: String,
}

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
            crate::error::Error::Database(format!("Could not save auth token: {}", e))
        })?;

        Ok(())
    }

    pub async fn get_by_token(db: &DbPool, auth_token: &str) -> Result<Uuid> {
        let query = sqlx::query!(
            r#"SELECT user_id FROM auth_token WHERE token = $1"#,
            auth_token,
        );

        let row = query.fetch_one(db).await.map_err(|e| {
            crate::error::Error::Database(format!("Could not get auth token: {}", e))
        })?;

        Ok(row.user_id)
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
            .map_err(|e| Error::Database(format!("Could not save user keys: {}", e)))?;

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
            .map_err(|e| Error::Database(format!("Could not get private_key {}", e)))?
            .map(|row| row.private_key);

        Ok(result)
    }

    pub async fn save_user_with_credentials(
        db: &DbPool,
        user_id: &Uuid,
        username: &str,
        password_hash: &str,
    ) -> Result<()> {
        let mut transaction = db
            .begin()
            .await
            .map_err(|e| Error::Database(format!("Could not start transaction {}", e)))?;

        let query = sqlx::query!(
            r#"INSERT INTO app_user (uuid, name) VALUES ($1, $2)"#,
            user_id,
            username,
        );
        query
            .execute(&mut *transaction)
            .await
            .map_err(|e| Error::Database(format!("Could not save user {}", e)))?;

        let query = sqlx::query!(
            r#"INSERT INTO user_account (
                user_id, account_id, password, provider
            ) VALUES ($1, $2, $3, $4)"#,
            user_id,
            username,
            password_hash,
            AccountProvider::Credentials as _,
        );
        query
            .execute(&mut *transaction)
            .await
            .map_err(|e| Error::Database(format!("Could not save user account {}", e)))?;

        transaction
            .commit()
            .await
            .map_err(|e| Error::Database(format!("Could not commit transaction {}", e)))?;

        Ok(())
    }

    pub async fn get_by_username(db: &DbPool, username: &str) -> Result<User> {
        let query = sqlx::query!(
            r#"SELECT user_id, password FROM user_account where account_id = $1 and provider = $2"#,
            username,
            AccountProvider::Credentials as _
        );

        let user = query
            .fetch_one(db)
            .await
            .map_err(|e| Error::Database(format!("Could not get user {}", e)))?;

        Ok(User {
            uuid: user.user_id,
            password: user
                .password
                .expect("Password must be set for credentials user"),
        })
    }
}
