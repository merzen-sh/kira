//! HTTP server configuration.

use std::net::SocketAddr;

use kira_core::secret::SecretString;
use serde::Deserialize;

use crate::ConfigError;
use crate::env;

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
    /// When unset, the server generates one at startup and prints it. The value
    /// is redacted from `Debug` output.
    #[serde(default)]
    pub password: Option<SecretString>,

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

impl ServerConfig {
    /// Load server configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns an error when `.env` cannot be loaded, `KIRA_GUILD_ID` is
    /// missing, or another `KIRA_*` variable has the wrong shape.
    pub fn from_env() -> Result<Self, ConfigError> {
        env::from_env("KIRA_")
    }

    /// Parse server configuration from an iterator of environment entries.
    ///
    /// # Errors
    ///
    /// Returns an error when `KIRA_GUILD_ID` is missing or another variable
    /// has the wrong shape.
    pub fn try_from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        env::try_from_iter("KIRA_", iter)
    }
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

pub(crate) fn default_bind() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 8080))
}

pub(crate) fn default_cli_bind() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 3000))
}

#[cfg(test)]
mod tests {
    use super::*;

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

        assert_eq!(
            c.password.as_ref().map(SecretString::expose),
            Some("secret")
        );
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

        assert!(c.password.is_none());
        assert_eq!(c.allowed_origins, None);
        assert!(!c.cookie_secure);
        assert!(!c.https);
        assert_eq!(c.cli_bind, SocketAddr::from(([127, 0, 0, 1], 3000)));
        assert_eq!(c.cookie_samesite, SameSite::Lax);
        Ok(())
    }
}
