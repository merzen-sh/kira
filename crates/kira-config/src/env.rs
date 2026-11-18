//! Environment and `.env` loading.

use serde::de::DeserializeOwned;

use crate::ConfigError;

/// Load variables from a nearby `.env` file, if one exists.
///
/// A missing `.env` is expected: deployments may inject variables directly.
/// Other failures (permissions, malformed lines) are returned to the caller.
///
/// # Errors
///
/// Returns an error when the file exists but cannot be read or parsed.
pub fn load_env() -> Result<(), ConfigError> {
    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
        Err(error) if error.not_found() => Ok(()),
        Err(error) => Err(sanitize_dotenv_error(error)),
    }
}

pub(crate) fn from_env<T: DeserializeOwned>(prefix: &str) -> Result<T, ConfigError> {
    load_env()?;
    try_from_iter(prefix, std::env::vars())
}

pub(crate) fn try_from_iter<T, I, K, V>(prefix: &str, iter: I) -> Result<T, ConfigError>
where
    T: DeserializeOwned,
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    envy::prefixed(prefix)
        .from_iter(iter.into_iter().map(|(k, v)| (k.into(), v.into())))
        .map_err(ConfigError::from)
}

fn sanitize_dotenv_error(error: dotenvy::Error) -> ConfigError {
    match error {
        // `LineParse` carries the raw line, which may contain a secret value.
        dotenvy::Error::LineParse(_, index) => ConfigError::DotenvLine { index },
        error => ConfigError::Dotenv(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
