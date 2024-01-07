use sqlx::{migrate, postgres::PgPoolOptions, Pool, Postgres};

use crate::{config::DatabaseConfig, error::Result};

pub type DbPool = Pool<Postgres>;

pub async fn init_db(config: &DatabaseConfig) -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.url)
        .await
        .map_err(|e| crate::error::Error::DbError(format!("Could not connect to db {}", e)))?;

    migrate!("db/migrations").run(&pool).await.map_err(|e| {
        crate::error::Error::DbMigrationError(format!("Could not run migrations {}", e))
    })?;

    Ok(pool)
}
