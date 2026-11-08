// The public config API intentionally exposes an inherent `from_iter` on each
// config type; it is not an implementation of `std::iter::FromIterator`.
#![allow(clippy::should_implement_trait)]

use std::net::SocketAddr;

use serde::Deserialize;
use thiserror::Error;

pub fn load_env() {
    let _ = dotenvy::dotenv();
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to parse environment variables: {0}")]
    Environment(#[from] envy::Error),
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: SocketAddr,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CliConfig {
    #[serde(default = "default_cli_bind", rename = "cli_bind")]
    pub bind: SocketAddr,
    #[serde(default = "default_remote_url")]
    pub remote_url: String,
}

fn default_bind() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 3000))
}

fn default_cli_bind() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 3001))
}

fn default_remote_url() -> String {
    "http://127.0.0.1:3000".to_owned()
}

impl DiscordConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        load_env();
        Self::from_iter(std::env::vars())
    }

    pub fn from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        envy::prefixed("DISCORD_")
            .from_iter(
                iter.into_iter()
                    .map(|(k, v)| (k.as_ref().to_owned(), v.into())),
            )
            .map_err(ConfigError::from)
    }
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        load_env();
        Self::from_iter(std::env::vars())
    }

    pub fn from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        envy::prefixed("KIRA_")
            .from_iter(
                iter.into_iter()
                    .map(|(k, v)| (k.as_ref().to_owned(), v.into())),
            )
            .map_err(ConfigError::from)
    }
}

impl CliConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        load_env();
        Self::from_iter(std::env::vars())
    }

    pub fn from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        envy::prefixed("KIRA_")
            .from_iter(
                iter.into_iter()
                    .map(|(k, v)| (k.as_ref().to_owned(), v.into())),
            )
            .map_err(ConfigError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_config_reads_token() {
        let c = DiscordConfig::from_iter([("DISCORD_TOKEN", "abc")]).unwrap();
        assert_eq!(c.token, "abc");
    }

    #[test]
    fn missing_token_is_an_error() {
        assert!(DiscordConfig::from_iter(std::iter::empty::<(&str, &str)>()).is_err());
    }

    #[test]
    fn server_config_defaults_bind() {
        let c = ServerConfig::from_iter(std::iter::empty::<(&str, &str)>()).unwrap();
        assert_eq!(c.bind, "127.0.0.1:3000".parse().unwrap());
    }

    #[test]
    fn server_config_reads_kira_bind() {
        let c = ServerConfig::from_iter([("KIRA_BIND", "0.0.0.0:8080")]).unwrap();
        assert_eq!(c.bind, "0.0.0.0:8080".parse().unwrap());
    }

    #[test]
    fn cli_config_defaults() {
        let c = CliConfig::from_iter(std::iter::empty::<(&str, &str)>()).unwrap();
        assert_eq!(c.bind, "127.0.0.1:3001".parse().unwrap());
        assert_eq!(c.remote_url, "http://127.0.0.1:3000");
    }

    #[test]
    fn cli_config_reads_env() {
        let c = CliConfig::from_iter([
            ("KIRA_CLI_BIND", "127.0.0.1:4001"),
            ("KIRA_REMOTE_URL", "http://example.test:9000"),
        ])
        .unwrap();
        assert_eq!(c.bind, "127.0.0.1:4001".parse().unwrap());
        assert_eq!(c.remote_url, "http://example.test:9000");
    }
}
