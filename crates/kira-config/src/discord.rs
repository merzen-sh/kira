//! Discord credentials configuration.

use kira_core::secret::SecretString;
use serde::Deserialize;

use crate::ConfigError;
use crate::env;

/// Discord credentials read from `DISCORD_*` variables.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DiscordConfig {
    /// Bot token used to authenticate with Discord.
    ///
    /// The token is redacted from `Debug` output.
    pub token: SecretString,
}

impl DiscordConfig {
    /// Load Discord configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns an error when `.env` cannot be loaded or `DISCORD_TOKEN` is
    /// missing.
    pub fn from_env() -> Result<Self, ConfigError> {
        env::from_env("DISCORD_")
    }

    /// Parse Discord configuration from an iterator of environment entries.
    ///
    /// # Errors
    ///
    /// Returns an error when `DISCORD_TOKEN` is missing or has the wrong shape.
    pub fn try_from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        env::try_from_iter("DISCORD_", iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_config_reads_token() -> Result<(), ConfigError> {
        let c = DiscordConfig::try_from_iter([("DISCORD_TOKEN", "abc")])?;
        assert_eq!(c.token.expose(), "abc");
        Ok(())
    }

    #[test]
    fn missing_token_is_an_error() {
        assert!(matches!(
            DiscordConfig::try_from_iter(std::iter::empty::<(&str, &str)>()),
            Err(ConfigError::Environment(_))
        ));
    }
}
