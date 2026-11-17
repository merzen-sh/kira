//! Request middleware and shared HTTP layers.

use std::net::{IpAddr, SocketAddr};

use axum::Json;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use kira_protocol::ErrorResponse;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::state::AppState;

/// Peer address captured from the TCP connection, when available.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct PeerAddress(pub(crate) Option<IpAddr>);

/// Record the connection peer for the login rate limiter.
pub(crate) async fn record_peer(mut request: Request, next: Next) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| connect.0.ip());
    request.extensions_mut().insert(PeerAddress(peer));
    next.run(request).await
}

/// Reject requests whose `Origin` header is not allowlisted.
pub(crate) async fn require_origin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());

    if origin.is_some_and(|origin| !state.auth().origin_allowed(origin)) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("origin is not allowed")),
        )
            .into_response();
    }

    next.run(request).await
}

/// Reject requests without a valid session cookie.
pub(crate) async fn require_session(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let authorized = state
        .auth()
        .token_from_headers(request.headers())
        .is_some_and(|token| state.auth().verify_token(&token));

    if authorized {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("authentication required")),
        )
            .into_response()
    }
}

pub(crate) fn cors_layer(state: &AppState) -> CorsLayer {
    let origins = state
        .auth()
        .allowed_origins()
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
        .allow_origin(AllowOrigin::list(origins))
        .max_age(std::time::Duration::from_secs(600))
}
