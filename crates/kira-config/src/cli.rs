//! Local dashboard configuration.

use std::net::SocketAddr;

use serde::Deserialize;

use crate::ConfigError;
use crate::env;
use crate::server::default_cli_bind;

/// Local dashboard settings read from `KIRA_*` variables.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct CliConfig {
    /// Address the local dashboard binds to (`KIRA_CLI_BIND`).
    #[serde(default = "default_cli_bind", rename = "cli_bind")]
    pub bind: SocketAddr,
}

impl CliConfig {
    /// Load local dashboard configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns an error when `.env` cannot be loaded or `KIRA_CLI_BIND` has the
    /// wrong shape.
    pub fn from_env() -> Result<Self, ConfigError> {
        env::from_env("KIRA_")
    }

    /// Parse local dashboard configuration from environment entries.
    ///
    /// # Errors
    ///
    /// Returns an error when a variable has the wrong shape.
    pub fn try_from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        env::try_from_iter("KIRA_", iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
