//! Health handler.

use axum::Json;
use kira_protocol::{ErrorResponse, HealthResponse};

/// Return the service health payload.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, body = HealthResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}
