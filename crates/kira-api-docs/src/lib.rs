//! Serialize the server's OpenAPI document for the committed contract.

use kira_server::ApiDoc;
use utoipa::OpenApi;

mod errors;

pub use errors::ApiDocsError;

/// Serialize the OpenAPI document to YAML.
///
/// # Errors
///
/// Returns an error when the document cannot be serialized.
pub fn yaml() -> Result<String, ApiDocsError> {
    ApiDoc::openapi()
        .to_yaml()
        .map_err(|error| ApiDocsError::Serialize(error.to_string()))
}
