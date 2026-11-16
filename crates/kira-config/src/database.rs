//! SQLite storage configuration.

use serde::Deserialize;

use crate::ConfigError;
use crate::env;

/// SQLite storage settings read from `KIRA_*` variables.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DatabaseConfig {
    /// SQLite connection URL (`KIRA_DATABASE_URL`).
    #[serde(default = "default_database_url", rename = "database_url")]
    pub url: String,
}

impl DatabaseConfig {
    /// Load SQLite storage configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns an error when `.env` cannot be loaded or a `KIRA_*` variable has
    /// the wrong shape.
    pub fn from_env() -> Result<Self, ConfigError> {
        env::from_env("KIRA_")
    }

    /// Parse SQLite storage configuration from environment entries.
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

fn default_database_url() -> String {
    "sqlite:kira.db".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

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
