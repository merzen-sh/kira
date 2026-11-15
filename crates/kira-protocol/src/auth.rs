//! Authentication request types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Credentials submitted to `POST /auth/login`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct LoginRequest {
    /// Dashboard PIN or password.
    pub pin: String,
}
