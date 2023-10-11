use sqlx::{postgres::PgPoolOptions, Pool, Postgres, migrate};

use crate::error::{Result, Error};

pub type DbPool = Pool<Postgres>;

pub async fn init_db() -> Result<DbPool> {
     let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:postgres@localhost:5432/photo_store_test")
        .await?;

    migrate!("db/migrations").run(&pool).await.map_err(|_| {
        // todo: improve error handling
        Error::Generic
    })?;

    Ok(pool)
}
