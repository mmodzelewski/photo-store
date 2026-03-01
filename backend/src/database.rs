use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::error;

use crate::{config::DatabaseConfig, error::Result, migration::Migrator};
use sea_orm_migration::MigratorTrait;

pub type DbPool = DatabaseConnection;

pub async fn init_db(config: &DatabaseConfig) -> Result<DbPool> {
    let mut opt = ConnectOptions::new(&config.url);
    opt.max_connections(5);

    let db = Database::connect(opt).await.map_err(|e| {
        error!(error = %e, "Could not connect to database");
        crate::error::Error::Database
    })?;

    Migrator::up(&db, None).await.map_err(|e| {
        error!(error = %e, "Could not run database migrations");
        crate::error::Error::DbMigration
    })?;

    Ok(db)
}
