use config::Environment;
use serde::Deserialize;
use tracing::info;

use crate::error::{Error, Result};

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    #[serde(rename = "database_url")]
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageConfig {
    #[serde(rename = "storage_url")]
    pub url: String,
    #[serde(rename = "storage_bucket_name")]
    pub bucket_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(flatten)]
    pub database: DatabaseConfig,
    #[serde(flatten)]
    pub storage: StorageConfig,
}

impl Config {
    pub fn load() -> Result<Self> {
        let settings = config::Config::builder()
            .add_source(Environment::with_prefix("app"))
            .build()
            .map_err(|e| Error::ConfigurationError(format!("Failed to load configuration {}", e)))?
            .try_deserialize()
            .map_err(|e| {
                Error::ConfigurationError(format!("Failed to deserialize configuration {}", e))
            })?;

        info!("Successfully loaded configuration");
        return Ok(settings);
    }
}
