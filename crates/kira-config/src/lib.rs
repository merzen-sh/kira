// The public config API intentionally exposes an inherent `from_iter` on each
// config type; it is not an implementation of `std::iter::FromIterator`.
#![allow(clippy::should_implement_trait)]

use std::net::SocketAddr;

use serde::Deserialize;
use thiserror::Error;

/// Load variables from a nearby `.env` file, if one exists.
///
/// A missing `.env` is expected: deployments may inject variables directly.
/// Other failures (permissions, malformed lines) are returned to the caller.
pub fn load_env() -> Result<(), ConfigError> {
    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
        Err(error) if error.not_found() => Ok(()),
        Err(error) => Err(sanitize_dotenv_error(error)),
    }
}

fn sanitize_dotenv_error(error: dotenvy::Error) -> ConfigError {
    match error {
        // `LineParse` carries the raw line, which may contain a secret value.
        dotenvy::Error::LineParse(_, index) => ConfigError::DotenvLine { index },
        error => ConfigError::Dotenv(error),
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to load .env file")]
    Dotenv(#[from] dotenvy::Error),

    #[error("failed to parse .env content near index {index}")]
    DotenvLine { index: usize },

    #[error("failed to parse environment variables")]
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
    SocketAddr::from(([127, 0, 0, 1], 8080))
}

fn default_cli_bind() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 3000))
}

fn default_remote_url() -> String {
    "http://127.0.0.1:8080".to_owned()
}

impl DiscordConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        load_env()?;
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
        load_env()?;
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
        load_env()?;
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
    fn discord_config_reads_token() -> Result<(), ConfigError> {
        let c = DiscordConfig::from_iter([("DISCORD_TOKEN", "abc")])?;
        assert_eq!(c.token, "abc");
        Ok(())
    }

    #[test]
    fn malformed_dotenv_line_is_sanitized() {
        let error = sanitize_dotenv_error(dotenvy::Error::LineParse(
            "DISCORD_TOKEN=super-secret".into(),
            4,
        ));

        assert_eq!(
            error.to_string(),
            "failed to parse .env content near index 4"
        );
        assert!(!error.to_string().contains("super-secret"));
    }

    #[test]
    fn missing_token_is_an_error() {
        assert!(matches!(
            DiscordConfig::from_iter(std::iter::empty::<(&str, &str)>()),
            Err(ConfigError::Environment(_))
        ));
    }

    #[test]
    fn server_config_defaults_bind() -> Result<(), ConfigError> {
        let c = ServerConfig::from_iter(std::iter::empty::<(&str, &str)>())?;
        assert_eq!(c.bind, SocketAddr::from(([127, 0, 0, 1], 8080)));
        Ok(())
    }

    #[test]
    fn server_config_reads_kira_bind() -> Result<(), ConfigError> {
        let c = ServerConfig::from_iter([("KIRA_BIND", "0.0.0.0:8080")])?;
        assert_eq!(c.bind, SocketAddr::from(([0, 0, 0, 0], 8080)));
        Ok(())
    }

    #[test]
    fn cli_config_defaults() -> Result<(), ConfigError> {
        let c = CliConfig::from_iter(std::iter::empty::<(&str, &str)>())?;
        assert_eq!(c.bind, SocketAddr::from(([127, 0, 0, 1], 3000)));
        assert_eq!(c.remote_url, "http://127.0.0.1:8080");
        Ok(())
    }

    #[test]
    fn cli_config_reads_env() -> Result<(), ConfigError> {
        let c = CliConfig::from_iter([
            ("KIRA_CLI_BIND", "127.0.0.1:4001"),
            ("KIRA_REMOTE_URL", "http://example.test:9000"),
        ])?;
        assert_eq!(c.bind, SocketAddr::from(([127, 0, 0, 1], 4001)));
        assert_eq!(c.remote_url, "http://example.test:9000");
        Ok(())
    }
}
