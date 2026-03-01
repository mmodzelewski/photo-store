use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::{config::DatabaseConfig, error::Result, migration::Migrator};
use sea_orm_migration::MigratorTrait;

pub type DbPool = DatabaseConnection;

pub async fn init_db(config: &DatabaseConfig) -> Result<DbPool> {
    let mut opt = ConnectOptions::new(&config.url);
    opt.max_connections(5);

    let db = Database::connect(opt)
        .await
        .map_err(|e| crate::error::Error::Database(format!("Could not connect to db {}", e)))?;

    Migrator::up(&db, None)
        .await
        .map_err(|e| crate::error::Error::DbMigration(format!("Could not run migrations {}", e)))?;

    Ok(db)
}
