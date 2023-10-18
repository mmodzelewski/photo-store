use uuid::Uuid;

use crate::{database::DbPool, error::Result};

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

    query.execute(db).await?;

    Ok(())
}
