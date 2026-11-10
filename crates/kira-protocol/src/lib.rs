//! Request and response types shared by the Kira server and its clients.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Health information returned by `GET /health`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct HealthResponse {
    /// Service status, `ok` when the server is healthy.
    pub status: String,
    /// Version of the running `kira` binary.
    pub version: String,
}

impl HealthResponse {
    /// Build the healthy response for this binary's version.
    #[must_use]
    pub fn ok() -> Self {
        Self {
            status: "ok".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

/// Error payload returned when a request fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct ErrorResponse {
    /// Human-readable failure message.
    pub error: String,
}

impl ErrorResponse {
    /// Build an error payload from a human-readable message.
    #[must_use]
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_ok_serializes_expected_fields() -> Result<(), Box<dyn std::error::Error>> {
        let v = serde_json::to_value(HealthResponse::ok())?;
        assert_eq!(v["status"], "ok");
        assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
        Ok(())
    }

    #[test]
    fn error_response_serializes() -> Result<(), Box<dyn std::error::Error>> {
        let v = serde_json::to_value(ErrorResponse::new("boom"))?;
        assert_eq!(v["error"], "boom");
        Ok(())
    }
}
