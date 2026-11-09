use thiserror::Error;
use utoipa::OpenApi;

/// Errors produced while assembling the OpenAPI document.
#[derive(Debug, Error)]
pub enum ApiDocsError {
    #[error("failed to serialize OpenAPI document: {0}")]
    Serialize(String),
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Kira API", version = env!("CARGO_PKG_VERSION"), description = "Kira service API"),
    tags((name = "health", description = "Service health")),
    paths(kira_server::health),
    components(schemas(kira_protocol::HealthResponse))
)]
pub struct ApiDoc;

pub fn yaml() -> Result<String, ApiDocsError> {
    ApiDoc::openapi()
        .to_yaml()
        .map_err(|error| ApiDocsError::Serialize(error.to_string()))
}
