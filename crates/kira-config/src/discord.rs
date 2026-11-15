//! Discord credentials configuration.

use serde::Deserialize;

use crate::ConfigError;
use crate::env;

/// Discord credentials read from `DISCORD_*` variables.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DiscordConfig {
    /// Bot token used to authenticate with Discord.
    pub token: String,
}

impl DiscordConfig {
    /// Load Discord configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        env::from_env("DISCORD_")
    }

    /// Parse Discord configuration from an iterator of environment entries.
    pub fn try_from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
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
        assert_eq!(c.token, "abc");
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
