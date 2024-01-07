use config::Environment;
use serde::Deserialize;
use tracing::{debug, info};

use crate::error::{Error, Result};

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageConfig {
    pub url: String,
    pub bucket_name: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
}

impl Config {
    pub fn load() -> Result<Self> {
        let database_config = config::Config::builder()
            .add_source(Environment::with_prefix("app_database"))
            .build()
            .map_err(|e| {
                Error::ConfigurationError(format!("Failed to load configuration {}", e))
            })?
        .try_deserialize().map_err(|e| {
            Error::ConfigurationError(format!("Failed to deserialize configuration {}", e))
        })?;

        let storage_config = config::Config::builder()
            .add_source(Environment::with_prefix("app_storage"))
            .build()
            .map_err(|e| {
                Error::ConfigurationError(format!("Failed to load configuration {}", e))
            })?
        .try_deserialize().map_err(|e| {
            Error::ConfigurationError(format!("Failed to deserialize configuration {}", e))
        })?;

        let settings = Self {
            database: database_config,
            storage: storage_config,
        };
        info!("Successfully loaded configuration");
        return Ok(settings);
    }
}
