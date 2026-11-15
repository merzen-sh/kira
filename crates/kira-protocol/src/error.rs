//! Error response types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
    fn error_response_serializes() -> Result<(), Box<dyn std::error::Error>> {
        let v = serde_json::to_value(ErrorResponse::new("boom"))?;
        assert_eq!(v["error"], "boom");
        Ok(())
    }
}
