//! Generic audit event handlers.

use axum::Json;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use kira_protocol::{AuditEventList, AuditFacets, ErrorResponse};
use serde::Deserialize;

use crate::errors::storage_error;
use crate::handlers::pagination;
use crate::state::AppState;

/// Query parameters for `GET /audit/events`.
#[derive(Debug, Deserialize)]
pub struct AuditListQuery {
    page: Option<u32>,
    page_limit: Option<u32>,
    search: Option<String>,
    action: Option<String>,
    actor_kind: Option<String>,
}

fn trimmed(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

/// List audit events, newest first.
#[utoipa::path(
    get,
    path = "/audit/events",
    tag = "audit",
    security(("cookie_auth" = [])),
    params(
        ("page" = Option<u32>, Query, description = "Page number, starting at 1"),
        ("page_limit" = Option<u32>, Query, description = "Events per page, 1-100 (default 20)"),
        ("search" = Option<String>, Query, description = "Case-insensitive substring of the action, actor, or target"),
        ("action" = Option<String>, Query, description = "Exact action filter"),
        ("actor_kind" = Option<String>, Query, description = "Exact actor kind filter")
    ),
    responses(
        (status = 200, body = AuditEventList),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn list_audit_events(
    State(state): State<AppState>,
    Query(query): Query<AuditListQuery>,
) -> Response {
    let (page, page_limit) = pagination(query.page, query.page_limit);
    let search = query.search.as_deref().and_then(trimmed);
    let action = query.action.as_deref().and_then(trimmed);
    let actor_kind = query.actor_kind.as_deref().and_then(trimmed);

    match state
        .database()
        .audit()
        .list_events(page, page_limit, search, action, actor_kind)
        .await
    {
        Ok(events) => Json(events).into_response(),
        Err(error) => storage_error(&error),
    }
}

/// Distinct actions and actor kinds available for filtering.
#[utoipa::path(
    get,
    path = "/audit/facets",
    tag = "audit",
    security(("cookie_auth" = [])),
    responses(
        (status = 200, body = AuditFacets),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn audit_facets(State(state): State<AppState>) -> Response {
    match state.database().audit().facets().await {
        Ok(facets) => Json(facets).into_response(),
        Err(error) => storage_error(&error),
    }
}
