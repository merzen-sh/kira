//! Server and authentication errors.

use std::net::SocketAddr;
use std::time::Duration;

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kira_protocol::ErrorResponse;
use kira_store::StoreError;
use thiserror::Error;

/// Errors produced while binding or serving the HTTP server.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServerError {
    /// The listener could not bind to the configured address.
    #[error("failed to bind server to {addr}")]
    Bind {
        /// Address the server tried to bind.
        addr: SocketAddr,
        /// Underlying bind failure.
        #[source]
        source: std::io::Error,
    },

    /// A non-loopback bind was requested without HTTPS, secure cookies, and
    /// an origin allowlist.
    #[error(
        "refusing to bind {addr} without KIRA_HTTPS=true, KIRA_COOKIE_SECURE=true, and KIRA_ALLOWED_ORIGINS"
    )]
    InsecureBind {
        /// Address the server refused to bind.
        addr: SocketAddr,
    },

    /// The dashboard binds beyond loopback without an origin allowlist.
    #[error("KIRA_ALLOWED_ORIGINS is required when the dashboard binds {addr}")]
    OriginsRequired {
        /// Address the dashboard host refused to derive origins for.
        addr: SocketAddr,
    },

    /// The configured origin allowlist is empty or malformed.
    #[error("KIRA_ALLOWED_ORIGINS must be a comma-separated list of http(s) origins")]
    InvalidOrigin,

    /// `SameSite=None` was requested without the `Secure` attribute.
    #[error("KIRA_COOKIE_SAMESITE=none requires KIRA_COOKIE_SECURE=true")]
    InsecureCookie,

    /// A random secret could not be generated.
    #[error("failed to generate a random secret")]
    Random,

    /// The server failed while accepting or serving connections.
    #[error("server I/O error")]
    Io(#[from] std::io::Error),
}

/// Why a login attempt was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoginError {
    /// The supplied PIN does not match.
    InvalidPin,
    /// Too many failures; retry after the reported delay.
    TooManyAttempts {
        /// How long the caller must wait.
        retry_after: Duration,
    },
}

/// Convert a storage failure into an opaque API response.
pub(crate) fn storage_error(error: &StoreError) -> Response {
    tracing::error!(error = ?error, "storage failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("storage failed")),
    )
        .into_response()
}
