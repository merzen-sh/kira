//! Errors produced while assembling the OpenAPI document.

use thiserror::Error;

/// Errors produced while assembling the OpenAPI document.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ApiDocsError {
    /// The document could not be serialized to YAML.
    #[error("failed to serialize OpenAPI document: {0}")]
    Serialize(String),
}
