use sqlx::{Pool, Postgres, migrate, postgres::PgPoolOptions};

use crate::{config::DatabaseConfig, error::Result};

pub type DbPool = Pool<Postgres>;

pub async fn init_db(config: &DatabaseConfig) -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.url)
        .await
        .map_err(|e| crate::error::Error::Database(format!("Could not connect to db {}", e)))?;

    migrate!("db/migrations")
        .run(&pool)
        .await
        .map_err(|e| crate::error::Error::DbMigration(format!("Could not run migrations {}", e)))?;

    Ok(pool)
}
