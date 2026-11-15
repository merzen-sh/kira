//! Host resource monitor handler.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kira_protocol::{ErrorResponse, MonitorReport};
use serde::Deserialize;

use crate::errors::storage_error;
use crate::monitor::unix_now;
use crate::state::AppState;

/// Query parameters for `GET /monitor`.
#[derive(Debug, Deserialize)]
pub struct MonitorQuery {
    range: Option<String>,
}

struct MonitorRange {
    name: &'static str,
    window: i64,
    bucket: i64,
}

const MONITOR_RANGES: [MonitorRange; 3] = [
    MonitorRange {
        name: "1h",
        window: 60 * 60,
        bucket: 30,
    },
    MonitorRange {
        name: "3h",
        window: 3 * 60 * 60,
        bucket: 60,
    },
    MonitorRange {
        name: "24h",
        window: 24 * 60 * 60,
        bucket: 5 * 60,
    },
];

fn monitor_range(value: Option<&str>) -> Option<&'static MonitorRange> {
    let value = value.unwrap_or("1h");
    MONITOR_RANGES.iter().find(|range| range.name == value)
}

/// Return current host resource usage and a bucketed history.
#[utoipa::path(
    get,
    path = "/monitor",
    tag = "monitor",
    security(("cookie_auth" = [])),
    params(
        ("range" = Option<String>, Query, description = "History range: `1h`, `3h`, or `24h` (default `1h`)")
    ),
    responses(
        (status = 200, body = MonitorReport),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn system_monitor(
    State(state): State<AppState>,
    Query(query): Query<MonitorQuery>,
) -> Response {
    let Some(range) = monitor_range(query.range.as_deref()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("range must be 1h, 3h, or 24h")),
        )
            .into_response();
    };

    let samples = state.database().monitor();
    let current = match samples.latest().await {
        Ok(current) => current,
        Err(error) => return storage_error(&error),
    };
    let since = unix_now() - range.window;
    let history = match samples.history(since, range.bucket).await {
        Ok(history) => history,
        Err(error) => return storage_error(&error),
    };

    Json(MonitorReport::new(range.name, current, history)).into_response()
}
