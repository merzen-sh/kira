//! Assemble the OpenAPI document from the server's route definitions.

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

/// OpenAPI document for the Kira service API.
#[derive(OpenApi)]
#[non_exhaustive]
#[openapi(
    info(title = "Kira API", version = env!("CARGO_PKG_VERSION"), description = "Kira service API"),
    tags((name = "health", description = "Service health")),
    paths(kira_server::health),
    components(schemas(kira_protocol::HealthResponse))
)]
pub struct ApiDoc;

/// Serialize the OpenAPI document to YAML.
pub fn yaml() -> Result<String, ApiDocsError> {
    ApiDoc::openapi()
        .to_yaml()
        .map_err(|error| ApiDocsError::Serialize(error.to_string()))
}
