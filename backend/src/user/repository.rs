use uuid::Uuid;

use crate::user::handlers::UserId;
use crate::{
    database::DbPool,
    error::{Error, Result},
};

pub(super) async fn save_user_with_credentials(
    db: &DbPool,
    user_id: &Uuid,
    user: &super::handlers::UserRegister,
) -> Result<()> {
    let mut transaction = db
        .begin()
        .await
        .map_err(|e| Error::DbError(format!("Could not start transaction {}", e)))?;
    let query = sqlx::query!(
        r#"INSERT INTO app_user (
                uuid, name
            ) VALUES ($1, $2)"#,
        user_id,
        user.username,
    );

    query
        .execute(&mut *transaction)
        .await
        .map_err(|e| Error::DbError(format!("Could not save user {}", e)))?;

    let query = sqlx::query!(
        r#"INSERT INTO user_account (
                user_id, account_id, password, provider
            ) VALUES ($1, $2, $3, $4)"#,
        user_id,
        user.username,
        user.password,
        AccountProvider::Credentials as _,
    );

    query
        .execute(&mut *transaction)
        .await
        .map_err(|e| Error::DbError(format!("Could not save user account {}", e)))?;

    transaction
        .commit()
        .await
        .map_err(|e| Error::DbError(format!("Could not commit transaction {}", e)))?;
    Ok(())
}

pub(super) async fn get_by_username(db: &DbPool, username: &str) -> Result<User> {
    let query = sqlx::query!(
        r#"SELECT user_id, password FROM user_account where account_id = $1 and provider = $2"#,
        username,
        AccountProvider::Credentials as _
    );

    let user = query
        .fetch_one(db)
        .await
        .map_err(|e| Error::DbError(format!("Could not get user {}", e)))?;

    Ok(User {
        uuid: user.user_id,
        username: username.to_string(),
        password: user
            .password
            .expect("Password must be set for credentials user"),
    })
}

pub(super) async fn find_by_provider(
    db: &DbPool,
    account_id: &str,
    provider: &AccountProvider,
) -> Result<Option<UserId>> {
    let query = sqlx::query!(
        r#"SELECT user_id FROM user_account where account_id = $1 and provider = $2"#,
        account_id,
        provider as _
    );
    let result = query
        .fetch_optional(db)
        .await
        .map_err(|e| Error::DbError(format!("Could not get user {}", e)))?;
    Ok(result.map(|r| UserId(r.user_id)))
}

pub(crate) async fn save_user_with_external_provider(
    db: &DbPool,
    user_id: &Uuid,
    account_id: &str,
    provider: &AccountProvider,
) -> Result<()> {
    let mut transaction = db
        .begin()
        .await
        .map_err(|e| Error::DbError(format!("Could not start transaction {}", e)))?;

    let query = sqlx::query!(r#"INSERT INTO app_user (uuid) VALUES ($1)"#, user_id);

    query
        .execute(&mut *transaction)
        .await
        .map_err(|e| Error::DbError(format!("Could not save user {}", e)))?;

    let query = sqlx::query!(
        r#"INSERT INTO user_account (user_id, account_id, provider) VALUES ($1, $2, $3)"#,
        user_id,
        account_id,
        provider as _,
    );

    query
        .execute(&mut *transaction)
        .await
        .map_err(|e| Error::DbError(format!("Could not save user account {}", e)))?;

    transaction
        .commit()
        .await
        .map_err(|e| Error::DbError(format!("Could not commit transaction {}", e)))?;

    Ok(())
}

pub(super) struct User {
    pub uuid: Uuid,
    pub username: String,
    pub password: String,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "provider")]
pub(crate) enum AccountProvider {
    Credentials,
    Google,
}
