//! Authentication handlers.

use std::net::IpAddr;

use axum::Json;
use axum::extract::{Extension, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kira_protocol::{AuditEntity, ErrorResponse, LoginRequest, SOURCE_DASHBOARD};
use kira_store::audit::NewAuditEvent;
use serde_json::Map;

use crate::errors::LoginError;
use crate::handlers::audit::{dashboard_actor, record};
use crate::middleware::PeerAddress;
use crate::state::AppState;

/// Action recorded after a successful dashboard login.
const ACTION_LOGIN: &str = "auth.login";
/// Action recorded when a dashboard login is rejected.
const ACTION_LOGIN_FAILED: &str = "auth.login.failed";

async fn record_login(state: &AppState, peer: Option<IpAddr>, action: &str, reason: Option<&str>) {
    let mut metadata = Map::new();
    if let Some(peer) = peer {
        metadata.insert("peer".to_owned(), peer.to_string().into());
    }
    if let Some(reason) = reason {
        metadata.insert("reason".to_owned(), reason.into());
    }

    let actor = if reason.is_some() {
        AuditEntity::new("anonymous", None, None)
    } else {
        dashboard_actor()
    };
    record(
        state.database(),
        &NewAuditEvent::new(action, SOURCE_DASHBOARD, &actor, None, &metadata),
    )
    .await;
}

/// Authenticate with the dashboard PIN and receive a session cookie.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 204, description = "Session cookie issued"),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 415, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
        (status = 429, body = ErrorResponse, headers(("Retry-After" = u64, description = "Seconds until another login attempt is allowed")))
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Extension(peer): Extension<PeerAddress>,
    Json(body): Json<LoginRequest>,
) -> Response {
    let mut response = match state.auth().login(&body.pin, peer.0) {
        Ok(cookie) => {
            record_login(&state, peer.0, ACTION_LOGIN, None).await;
            (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response()
        }
        Err(LoginError::InvalidPin) => {
            record_login(&state, peer.0, ACTION_LOGIN_FAILED, Some("invalid_pin")).await;
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("invalid PIN")),
            )
                .into_response()
        }
        Err(LoginError::TooManyAttempts { retry_after }) => {
            record_login(&state, peer.0, ACTION_LOGIN_FAILED, Some("rate_limited")).await;
            let seconds = retry_after.as_secs().max(1);
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse::new("too many login attempts")),
            )
                .into_response();

            if let Ok(value) = HeaderValue::from_str(&seconds.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }

            response
        }
    };

    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// Clear the session cookie.
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = 204, description = "Session cookie cleared"),
        (status = 403, body = ErrorResponse)
    )
)]
pub async fn logout(State(state): State<AppState>) -> Response {
    (
        StatusCode::NO_CONTENT,
        [
            (header::SET_COOKIE, state.auth().clear_cookie()),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
    )
        .into_response()
}

/// Report whether the request carries a valid session.
#[utoipa::path(
    get,
    path = "/auth/session",
    tag = "auth",
    security(("cookie_auth" = [])),
    responses(
        (status = 204, description = "Session is valid"),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub async fn session() -> StatusCode {
    StatusCode::NO_CONTENT
}
