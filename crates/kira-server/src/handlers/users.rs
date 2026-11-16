//! Guild user handlers.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kira_protocol::{ErrorResponse, GuildUserList, GuildUserStats};
use serde::Deserialize;

use crate::errors::storage_error;
use crate::handlers::pagination;
use crate::state::AppState;

/// Query parameters for `GET /guild/users`.
#[derive(Debug, Deserialize)]
pub struct UserListQuery {
    page: Option<u32>,
    page_limit: Option<u32>,
    search: Option<String>,
    status: Option<String>,
    kind: Option<String>,
}

/// List tracked guild users, newest join first.
#[utoipa::path(
    get,
    path = "/guild/users",
    tag = "guild",
    security(("cookie_auth" = [])),
    params(
        ("page" = Option<u32>, Query, description = "Page number, starting at 1"),
        ("page_limit" = Option<u32>, Query, description = "Users per page, 1-100 (default 20)"),
        ("search" = Option<String>, Query, description = "Case-insensitive substring of the user ID, username, or display name"),
        ("status" = Option<String>, Query, description = "Membership filter: `active` or `left`"),
        ("kind" = Option<String>, Query, description = "Account filter: `user`, `bot`, or `all` (default `all`)")
    ),
    responses(
        (status = 200, body = GuildUserList),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn list_guild_users(
    State(state): State<AppState>,
    Query(query): Query<UserListQuery>,
) -> Response {
    let (page, page_limit) = pagination(query.page, query.page_limit);
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(status) = status
        && !matches!(status, "active" | "left")
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("status must be active or left")),
        )
            .into_response();
    }

    let kind = query
        .kind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let is_bot = match kind {
        None | Some("all") => None,
        Some("user") => Some(false),
        Some("bot") => Some(true),
        Some(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("kind must be user, bot, or all")),
            )
                .into_response();
        }
    };

    match state
        .database()
        .users()
        .list(page, page_limit, search, status, is_bot)
        .await
    {
        Ok(users) => Json(users).into_response(),
        Err(error) => storage_error(&error),
    }
}

/// Guild user totals for the dashboard summary cards.
#[utoipa::path(
    get,
    path = "/guild/users/stats",
    tag = "guild",
    security(("cookie_auth" = [])),
    responses(
        (status = 200, body = GuildUserStats),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn guild_user_stats(State(state): State<AppState>) -> Response {
    match state.database().users().stats().await {
        Ok(stats) => Json(stats).into_response(),
        Err(error) => storage_error(&error),
    }
}
