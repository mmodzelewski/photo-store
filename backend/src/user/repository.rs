use uuid::Uuid;

use crate::{
    database::DbPool,
    error::{Error, Result},
};

pub(super) async fn save(
    db: &DbPool,
    user_id: &Uuid,
    user: &super::handlers::UserRegister,
) -> Result<()> {
    let query = sqlx::query!(
        r#"INSERT INTO app_user (
                uuid, username, password
            ) VALUES ($1, $2, $3)"#,
        user_id,
        user.username,
        user.password,
    );

    query
        .execute(db)
        .await
        .map_err(|e| Error::DbError(format!("Could not save user {}", e)))?;

    Ok(())
}

pub(super) async fn get_by_username(db: &DbPool, username: &str) -> Result<User> {
    let query = sqlx::query_as!(
        User,
        r#"SELECT uuid, username, password FROM app_user where username = $1"#,
        username
    );
    let user = query
        .fetch_one(db)
        .await
        .map_err(|e| Error::DbError(format!("Could not get user {}", e)))?;
    Ok(user)
}

pub(super) struct User {
    pub uuid: Uuid,
    pub username: String,
    pub password: String,
}
