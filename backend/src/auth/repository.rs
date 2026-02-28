use crate::database::DbPool;
use crate::error::{Error, Result};
use crate::ulid::Id;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "provider")]
pub(super) enum AccountProvider {
    Credentials,
}

pub(super) struct User {
    pub id: Id,
    pub password: String,
}

pub(super) struct AuthRepository;

impl AuthRepository {
    pub async fn save_auth_token(db: &DbPool, user_id: &Id, auth_token: &str) -> Result<()> {
        let token_id = Id::new();
        let query = sqlx::query!(
            r#"INSERT INTO auth_token (
                id, user_id, token
            ) VALUES ($1, $2, $3)"#,
            &token_id as &Id,
            user_id as &Id,
            auth_token,
        );

        query.execute(db).await.map_err(|e| {
            crate::error::Error::Database(format!("Could not save auth token: {}", e))
        })?;

        Ok(())
    }

    pub async fn get_by_token(db: &DbPool, auth_token: &str) -> Result<Id> {
        let query = sqlx::query!(
            r#"SELECT user_id as "user_id: Id" FROM auth_token WHERE token = $1"#,
            auth_token,
        );

        let row = query.fetch_one(db).await.map_err(|e| {
            crate::error::Error::Database(format!("Could not get auth token: {}", e))
        })?;

        Ok(row.user_id)
    }

    pub(crate) async fn save_keys(
        db: &sqlx::Pool<sqlx::Postgres>,
        user_id: &Id,
        private_key: &str,
        public_key: &str,
    ) -> Result<()> {
        let keys_id = Id::new();
        let query = sqlx::query!(
            r#"INSERT INTO user_keys (
                id, user_id, private_key, public_key
            ) VALUES ($1, $2, $3, $4)"#,
            &keys_id as &Id,
            user_id as &Id,
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
        user_id: &Id,
    ) -> Result<Option<String>> {
        let query = sqlx::query!(
            r#"SELECT private_key FROM user_keys where user_id = $1"#,
            user_id as &Id
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
        user_id: &Id,
        username: &str,
        password_hash: &str,
    ) -> Result<()> {
        let mut transaction = db
            .begin()
            .await
            .map_err(|e| Error::Database(format!("Could not start transaction {}", e)))?;

        let query = sqlx::query!(
            r#"INSERT INTO app_user (id, name) VALUES ($1, $2)"#,
            user_id as &Id,
            username,
        );
        query
            .execute(&mut *transaction)
            .await
            .map_err(|e| Error::Database(format!("Could not save user {}", e)))?;

        let account_id = Id::new();
        let query = sqlx::query!(
            r#"INSERT INTO user_account (
                id, user_id, account_id, password, provider
            ) VALUES ($1, $2, $3, $4, $5)"#,
            &account_id as &Id,
            user_id as &Id,
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
            r#"SELECT user_id as "user_id: Id", password FROM user_account where account_id = $1 and provider = $2"#,
            username,
            AccountProvider::Credentials as _
        );

        let user = query
            .fetch_one(db)
            .await
            .map_err(|e| Error::Database(format!("Could not get user {}", e)))?;

        Ok(User {
            id: user.user_id,
            password: user
                .password
                .expect("Password must be set for credentials user"),
        })
    }
}
