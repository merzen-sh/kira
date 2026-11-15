//! Errors produced while loading configuration.

use thiserror::Error;

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
