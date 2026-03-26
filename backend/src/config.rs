use config::Environment;
use serde::Deserialize;
use tracing::{error, info};

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

fn default_max_file_size() -> i64 {
    10 * 1024 * 1024 * 1024 // 10 GiB
}
fn default_session_ttl_hours() -> i64 {
    168
}
fn default_presigned_url_ttl_secs() -> u64 {
    3600
}
fn default_gc_interval_secs() -> u64 {
    3600
}
fn default_max_concurrent_sessions() -> i64 {
    10
}

#[derive(Debug, Deserialize, Clone)]
pub struct UploadConfig {
    #[serde(rename = "upload_max_file_size", default = "default_max_file_size")]
    pub max_file_size: i64,

    #[serde(
        rename = "upload_session_ttl_hours",
        default = "default_session_ttl_hours"
    )]
    pub session_ttl_hours: i64,

    #[serde(
        rename = "upload_presigned_url_ttl_secs",
        default = "default_presigned_url_ttl_secs"
    )]
    pub presigned_url_ttl_secs: u64,

    #[serde(
        rename = "upload_gc_interval_secs",
        default = "default_gc_interval_secs"
    )]
    pub gc_interval_secs: u64,

    #[serde(
        rename = "upload_max_concurrent_sessions",
        default = "default_max_concurrent_sessions"
    )]
    pub max_concurrent_sessions: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(flatten)]
    pub database: DatabaseConfig,
    #[serde(flatten)]
    pub storage: StorageConfig,
    #[serde(flatten)]
    pub upload: UploadConfig,
    #[serde(default)]
    pub registration_enabled: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        let settings = config::Config::builder()
            .add_source(Environment::with_prefix("app"))
            .build()
            .map_err(|e| {
                error!(error = %e, "Failed to load configuration");
                Error::Configuration
            })?
            .try_deserialize()
            .map_err(|e| {
                error!(error = %e, "Failed to deserialize configuration");
                Error::Configuration
            })?;

        info!("Successfully loaded configuration");
        Ok(settings)
    }
}
