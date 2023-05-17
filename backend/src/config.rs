use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

use anyhow::Context;
use tokio::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub r2_url: String,
    pub bucket_name: String,
}

impl Config {
    pub async fn load() -> Result<Config> {
        let config = fs::read_to_string(Path::new("config.toml")).await?;
        return toml::from_str(&config).context("Cannot parse config file");
    }
}
