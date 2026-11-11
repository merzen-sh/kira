//! Serialize the server's OpenAPI document for the committed contract.

use kira_server::ApiDoc;
use thiserror::Error;
use utoipa::OpenApi;

/// Errors produced while assembling the OpenAPI document.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ApiDocsError {
    /// The document could not be serialized to YAML.
    #[error("failed to serialize OpenAPI document: {0}")]
    Serialize(String),
}

/// Serialize the OpenAPI document to YAML.
pub fn yaml() -> Result<String, ApiDocsError> {
    ApiDoc::openapi()
        .to_yaml()
        .map_err(|error| ApiDocsError::Serialize(error.to_string()))
}
