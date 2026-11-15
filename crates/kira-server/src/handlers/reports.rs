//! Bug report handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kira_protocol::{ErrorResponse, Report, ReportList, ReportStats, UpdateReport};
use serde::Deserialize;

use crate::errors::storage_error;
use crate::handlers::pagination;
use crate::state::AppState;

/// Query parameters for `GET /reports`.
#[derive(Debug, Deserialize)]
pub struct ReportListQuery {
    page: Option<u32>,
    page_limit: Option<u32>,
    search: Option<String>,
}

/// List bug reports, newest first.
#[utoipa::path(
    get,
    path = "/reports",
    tag = "reports",
    security(("cookie_auth" = [])),
    params(
        ("page" = Option<u32>, Query, description = "Page number, starting at 1"),
        ("page_limit" = Option<u32>, Query, description = "Reports per page, 1-100 (default 20)"),
        ("search" = Option<String>, Query, description = "Case-insensitive substring of the title, steps, details, or reporter ID")
    ),
    responses(
        (status = 200, body = ReportList),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn list_reports(
    State(state): State<AppState>,
    Query(query): Query<ReportListQuery>,
) -> Response {
    let (page, page_limit) = pagination(query.page, query.page_limit);
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match state
        .database()
        .reports()
        .list_reports(page, page_limit, search)
        .await
    {
        Ok(reports) => Json(reports).into_response(),
        Err(error) => storage_error(&error),
    }
}

/// Report totals for the dashboard summary cards.
#[utoipa::path(
    get,
    path = "/reports/stats",
    tag = "reports",
    security(("cookie_auth" = [])),
    responses(
        (status = 200, body = ReportStats),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn report_stats(State(state): State<AppState>) -> Response {
    match state.database().reports().report_stats().await {
        Ok(stats) => Json(stats).into_response(),
        Err(error) => storage_error(&error),
    }
}

/// Mark a bug report resolved or unresolved.
#[utoipa::path(
    patch,
    path = "/reports/{id}",
    tag = "reports",
    security(("cookie_auth" = [])),
    params(("id" = i64, Path, description = "Report ID")),
    request_body = UpdateReport,
    responses(
        (status = 200, body = Report),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 415, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn update_report(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateReport>,
) -> Response {
    match state
        .database()
        .reports()
        .set_resolved(id, body.resolved)
        .await
    {
        Ok(Some(report)) => Json(report).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("report was not found")),
        )
            .into_response(),
        Err(error) => storage_error(&error),
    }
}

/// Delete a bug report.
#[utoipa::path(
    delete,
    path = "/reports/{id}",
    tag = "reports",
    security(("cookie_auth" = [])),
    params(("id" = i64, Path, description = "Report ID")),
    responses(
        (status = 204, description = "Report deleted"),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn delete_report(State(state): State<AppState>, Path(id): Path<i64>) -> Response {
    match state.database().reports().delete_report(id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("report was not found")),
        )
            .into_response(),
        Err(error) => storage_error(&error),
    }
}
