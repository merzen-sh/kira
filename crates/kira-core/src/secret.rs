//! Strings that keep their value out of logs.

use std::fmt;

use serde::Deserialize;

/// A configuration secret whose value is hidden from `Debug` output.
///
/// The value is only reachable through [`SecretString::expose`], so logging a
/// configuration struct cannot leak it by accident.
#[derive(Clone, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    /// Wrap a secret value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the secret value at the boundary that needs it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::SecretString;

    #[test]
    fn debug_output_is_redacted() {
        let secret = SecretString::new("super-secret");

        assert_eq!(secret.expose(), "super-secret");
        assert_eq!(format!("{secret:?}"), "SecretString(<redacted>)");
    }
}
