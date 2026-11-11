//! Environment-backed configuration for Kira binaries.
//!
//! Provides configuration types for Discord, the HTTP server, and the local
//! dashboard. Configuration can be loaded from process environment variables
//! or parsed from an iterator for testing.
//!
//! Environment variables use the following prefixes:
//! - `DISCORD_` for Discord configuration.
//! - `KIRA_` for server and CLI configuration.

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

/// Errors produced while loading or parsing configuration.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// A `.env` file was found but could not be loaded.
    #[error("failed to load .env file")]
    Dotenv(#[from] dotenvy::Error),

    /// A line in `.env` could not be parsed.
    #[error("failed to parse .env content near index {index}")]
    DotenvLine {
        /// Byte offset near the malformed line.
        index: usize,
    },

    /// An environment variable was missing or had the wrong shape.
    #[error("failed to parse environment variables")]
    Environment(#[from] envy::Error),
}

/// Discord credentials read from `DISCORD_*` variables.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DiscordConfig {
    /// Bot token used to authenticate with Discord.
    pub token: String,
}

/// HTTP server settings read from `KIRA_*` variables.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ServerConfig {
    /// Address the HTTP server binds to.
    #[serde(default = "default_bind")]
    pub bind: SocketAddr,
}

/// Local dashboard settings read from `KIRA_*` variables.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct CliConfig {
    /// Address the local dashboard binds to (`KIRA_CLI_BIND`).
    #[serde(default = "default_cli_bind", rename = "cli_bind")]
    pub bind: SocketAddr,

    /// Base URL of the remote Kira server (`KIRA_REMOTE_URL`).
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
    /// Load Discord configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        load_env()?;
        Self::try_from_iter(std::env::vars())
    }

    /// Parse Discord configuration from an iterator of environment entries.
    pub fn try_from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>
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
    /// Load server configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        load_env()?;
        Self::try_from_iter(std::env::vars())
    }

    /// Parse server configuration from an iterator of environment entries.
    pub fn try_from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>
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
    /// Load local dashboard configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        load_env()?;
        Self::try_from_iter(std::env::vars())
    }

    /// Parse local dashboard configuration from environment entries.
    pub fn try_from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>
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
        let c = DiscordConfig::try_from_iter([("DISCORD_TOKEN", "abc")])?;
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
            DiscordConfig::try_from_iter(std::iter::empty::<(&str, &str)>()),
            Err(ConfigError::Environment(_))
        ));
    }

    #[test]
    fn server_config_defaults_bind() -> Result<(), ConfigError> {
        let c = ServerConfig::try_from_iter(std::iter::empty::<(&str, &str)>())?;

        assert_eq!(c.bind, SocketAddr::from(([127, 0, 0, 1], 8080)));
        Ok(())
    }

    #[test]
    fn server_config_reads_kira_bind() -> Result<(), ConfigError> {
        let c = ServerConfig::try_from_iter([("KIRA_BIND", "0.0.0.0:8080")])?;

        assert_eq!(c.bind, SocketAddr::from(([0, 0, 0, 0], 8080)));
        Ok(())
    }

    #[test]
    fn cli_config_defaults() -> Result<(), ConfigError> {
        let c = CliConfig::try_from_iter(std::iter::empty::<(&str, &str)>())?;

        assert_eq!(c.bind, SocketAddr::from(([127, 0, 0, 1], 3000)));
        assert_eq!(c.remote_url, "http://127.0.0.1:8080");
        Ok(())
    }

    #[test]
    fn cli_config_reads_env() -> Result<(), ConfigError> {
        let c = CliConfig::try_from_iter([
            ("KIRA_CLI_BIND", "127.0.0.1:4001"),
            ("KIRA_REMOTE_URL", "http://example.test:9000"),
        ])?;

        assert_eq!(c.bind, SocketAddr::from(([127, 0, 0, 1], 4001)));
        assert_eq!(c.remote_url, "http://example.test:9000");
        Ok(())
    }
}
