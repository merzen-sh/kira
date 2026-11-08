use dotenvy::dotenv;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub discord_token: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load .env file: {0}")]
    Dotenv(#[from] dotenvy::Error),

    #[error("Failed to parse environment variables: {0}")]
    Environment(#[from] envy::Error),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenv()?;

        envy::from_env().map_err(ConfigError::from)
    }
}
