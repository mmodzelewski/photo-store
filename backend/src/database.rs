use sqlx::{migrate, postgres::PgPoolOptions, Pool, Postgres};

use crate::error::Result;

pub type DbPool = Pool<Postgres>;

pub async fn init_db() -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:postgres@localhost:5432/photo_store_test")
        .await?;

    migrate!("db/migrations").run(&pool).await?;

    Ok(pool)
}
