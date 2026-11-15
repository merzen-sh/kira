//! Authentication handlers.

use axum::Json;
use axum::extract::{Extension, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kira_protocol::{ErrorResponse, LoginRequest};

use crate::errors::LoginError;
use crate::middleware::PeerAddress;
use crate::state::AppState;

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
        Ok(cookie) => (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response(),
        Err(LoginError::InvalidPin) => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("invalid PIN")),
        )
            .into_response(),
        Err(LoginError::TooManyAttempts { retry_after }) => {
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
