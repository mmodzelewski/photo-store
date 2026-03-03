use anyhow::Context;
use serde::Deserialize;
use std::{fmt::Display, str::FromStr};
use strum::{Display, EnumString};

#[derive(Debug, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum ThumbnailMode {
    Cover,
    Contain,
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
pub struct ThumbnailVariant {
    max_size: u32,
    mode: ThumbnailMode,
}

impl TryFrom<String> for ThumbnailVariant {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl ThumbnailVariant {
    fn cover(max_size: u32) -> Self {
        Self {
            max_size,
            mode: ThumbnailMode::Cover,
        }
    }

    fn contain(max_size: u32) -> Self {
        Self {
            max_size,
            mode: ThumbnailMode::Contain,
        }
    }

    pub fn small_cover() -> Self {
        Self::cover(512)
    }

    pub fn big_contain() -> Self {
        Self::contain(1920)
    }
}

impl Default for ThumbnailVariant {
    fn default() -> Self {
        Self {
            max_size: 1920,
            mode: ThumbnailMode::Contain,
        }
    }
}

impl Display for ThumbnailVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.max_size, self.mode)
    }
}

impl FromStr for ThumbnailVariant {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (max_size, mode) = s
            .split_once('-')
            .context("Cannot split ThumbnailVariant string")?;
        let max_size = max_size
            .parse()
            .context(format!("Cannot parse {} to u32", max_size))?;
        let mode = mode
            .parse()
            .context(format!("Cannot parse {} to ThumbnailMode", mode))?;
        Ok(Self { max_size, mode })
    }
}
