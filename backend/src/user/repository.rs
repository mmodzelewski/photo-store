use crate::{database::DbPool, error::Result};

pub(super) async fn save(db: &DbPool, file: &super::handlers::UserRegister) -> Result<()> {
    let query = sqlx::query!(
        r#"INSERT INTO app_user (
                username, password
            ) VALUES ($1, $2)"#,
        file.username,
        file.password,
    );

    query.execute(db).await?;

    Ok(())
}
