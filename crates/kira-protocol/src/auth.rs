//! Authentication request types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Credentials submitted to `POST /auth/login`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct LoginRequest {
    /// Dashboard PIN or password.
    pub pin: String,
}

#[cfg(test)]
mod tests {
    use super::LoginRequest;

    #[test]
    fn unknown_login_fields_are_rejected() {
        assert!(serde_json::from_str::<LoginRequest>(r#"{"pin":"1","extra":true}"#).is_err());
    }
}
