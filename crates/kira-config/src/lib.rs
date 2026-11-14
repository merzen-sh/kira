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

    /// Discord server exposed by the API (`KIRA_GUILD_ID`).
    pub guild_id: u64,

    /// Dashboard PIN or password (`KIRA_PASSWORD`).
    ///
    /// When unset, the server generates one at startup and prints it.
    #[serde(default)]
    pub password: Option<String>,

    /// Comma-separated dashboard origins allowed to call the API
    /// (`KIRA_ALLOWED_ORIGINS`).
    ///
    /// Required for non-loopback binds. Loopback deployments default to the
    /// dashboard host address plus the Vite dev server origins.
    #[serde(default)]
    pub allowed_origins: Option<String>,

    /// Set the `Secure` attribute on the session cookie (`KIRA_COOKIE_SECURE`).
    #[serde(default)]
    pub cookie_secure: bool,

    /// Assert that the server is served over HTTPS (`KIRA_HTTPS`).
    ///
    /// TLS is terminated by a reverse proxy; this flag makes that deployment
    /// explicit. Required for non-loopback binds.
    #[serde(default)]
    pub https: bool,

    /// Address the dashboard host binds to (`KIRA_CLI_BIND`).
    ///
    /// Used to derive default allowed origins for loopback deployments.
    #[serde(default = "default_cli_bind", rename = "cli_bind")]
    pub cli_bind: SocketAddr,

    /// `SameSite` policy for the session cookie (`KIRA_COOKIE_SAMESITE`).
    #[serde(default)]
    pub cookie_samesite: SameSite,
}

/// SQLite storage settings read from `KIRA_*` variables.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DatabaseConfig {
    /// SQLite connection URL (`KIRA_DATABASE_URL`).
    #[serde(default = "default_database_url", rename = "database_url")]
    pub url: String,
}

/// `SameSite` policy for the session cookie.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SameSite {
    /// `SameSite=Lax`: sent on same-site requests and top-level navigations.
    #[default]
    Lax,
    /// `SameSite=Strict`: sent only on same-site requests.
    Strict,
    /// `SameSite=None`: sent on cross-site requests; requires `Secure`.
    None,
}

impl std::fmt::Display for SameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Lax => "Lax",
            Self::Strict => "Strict",
            Self::None => "None",
        };
        f.write_str(value)
    }
}

/// Local dashboard settings read from `KIRA_*` variables.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct CliConfig {
    /// Address the local dashboard binds to (`KIRA_CLI_BIND`).
    #[serde(default = "default_cli_bind", rename = "cli_bind")]
    pub bind: SocketAddr,
}

fn default_bind() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 8080))
}

fn default_cli_bind() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 3000))
}

fn default_database_url() -> String {
    "sqlite:kira.db".to_owned()
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

impl DatabaseConfig {
    /// Load report storage configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        load_env()?;
        Self::try_from_iter(std::env::vars())
    }

    /// Parse report storage configuration from environment entries.
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
        let c = ServerConfig::try_from_iter([("KIRA_GUILD_ID", "1")])?;

        assert_eq!(c.bind, SocketAddr::from(([127, 0, 0, 1], 8080)));
        Ok(())
    }

    #[test]
    fn server_config_requires_a_guild_id() {
        assert!(matches!(
            ServerConfig::try_from_iter(std::iter::empty::<(&str, &str)>()),
            Err(ConfigError::Environment(_))
        ));
    }

    #[test]
    fn server_config_reads_kira_bind() -> Result<(), ConfigError> {
        let c =
            ServerConfig::try_from_iter([("KIRA_BIND", "0.0.0.0:8080"), ("KIRA_GUILD_ID", "1")])?;

        assert_eq!(c.bind, SocketAddr::from(([0, 0, 0, 0], 8080)));
        Ok(())
    }

    #[test]
    fn server_config_reads_guild_id() -> Result<(), ConfigError> {
        let c = ServerConfig::try_from_iter([("KIRA_GUILD_ID", "123456789012345678")])?;

        assert_eq!(c.guild_id, 123_456_789_012_345_678);
        Ok(())
    }

    #[test]
    fn server_config_reads_password_and_cookie_settings() -> Result<(), ConfigError> {
        let c = ServerConfig::try_from_iter([
            ("KIRA_GUILD_ID", "1"),
            ("KIRA_PASSWORD", "secret"),
            ("KIRA_ALLOWED_ORIGINS", "http://127.0.0.1:3000"),
            ("KIRA_COOKIE_SECURE", "true"),
            ("KIRA_CLI_BIND", "127.0.0.1:4000"),
            ("KIRA_HTTPS", "true"),
            ("KIRA_COOKIE_SAMESITE", "strict"),
        ])?;

        assert_eq!(c.password.as_deref(), Some("secret"));
        assert_eq!(c.allowed_origins.as_deref(), Some("http://127.0.0.1:3000"));
        assert!(c.cookie_secure);
        assert!(c.https);
        assert_eq!(c.cli_bind, SocketAddr::from(([127, 0, 0, 1], 4000)));
        assert_eq!(c.cookie_samesite, SameSite::Strict);
        Ok(())
    }

    #[test]
    fn server_config_defaults_cookie_settings() -> Result<(), ConfigError> {
        let c = ServerConfig::try_from_iter([("KIRA_GUILD_ID", "1")])?;

        assert_eq!(c.password, None);
        assert_eq!(c.allowed_origins, None);
        assert!(!c.cookie_secure);
        assert!(!c.https);
        assert_eq!(c.cli_bind, SocketAddr::from(([127, 0, 0, 1], 3000)));
        assert_eq!(c.cookie_samesite, SameSite::Lax);
        Ok(())
    }

    #[test]
    fn cli_config_defaults() -> Result<(), ConfigError> {
        let c = CliConfig::try_from_iter(std::iter::empty::<(&str, &str)>())?;

        assert_eq!(c.bind, SocketAddr::from(([127, 0, 0, 1], 3000)));
        Ok(())
    }

    #[test]
    fn cli_config_reads_bind() -> Result<(), ConfigError> {
        let c = CliConfig::try_from_iter([("KIRA_CLI_BIND", "127.0.0.1:4001")])?;

        assert_eq!(c.bind, SocketAddr::from(([127, 0, 0, 1], 4001)));
        Ok(())
    }

    #[test]
    fn database_config_defaults_to_a_local_file() -> Result<(), ConfigError> {
        let c = DatabaseConfig::try_from_iter(std::iter::empty::<(&str, &str)>())?;

        assert_eq!(c.url, "sqlite:kira.db");
        Ok(())
    }

    #[test]
    fn database_config_reads_url() -> Result<(), ConfigError> {
        let c = DatabaseConfig::try_from_iter([("KIRA_DATABASE_URL", "sqlite:reports.db")])?;

        assert_eq!(c.url, "sqlite:reports.db");
        Ok(())
    }
}
