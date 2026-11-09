use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

impl HealthResponse {
    pub fn ok() -> Self {
        Self {
            status: "ok".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
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
        let v = serde_json::to_value(ErrorResponse {
            error: "boom".into(),
        })?;
        assert_eq!(v["error"], "boom");
        Ok(())
    }
}
