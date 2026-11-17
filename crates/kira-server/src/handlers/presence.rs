//! Bot presence handlers.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kira_protocol::{AuditEntity, BotPresence, ErrorResponse, SOURCE_DASHBOARD};
use kira_store::audit::NewAuditEvent;
use serde_json::Map;

use crate::errors::storage_error;
use crate::handlers::audit::{dashboard_actor, record};
use crate::state::AppState;

/// Action recorded when a dashboard admin changes the bot presence.
const ACTION_PRESENCE_UPDATED: &str = "bot.presence.updated";

/// Discord's maximum activity text length, in characters.
const MAX_ACTIVITY_TEXT: usize = 128;

/// Return the stored bot presence configuration.
#[utoipa::path(
    get,
    path = "/bot/presence",
    tag = "bot",
    security(("cookie_auth" = [])),
    responses(
        (status = 200, body = BotPresence),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn get_presence(State(state): State<AppState>) -> Response {
    match state.database().presence().get().await {
        Ok(presence) => Json(presence).into_response(),
        Err(error) => storage_error(&error),
    }
}

/// Save the bot presence configuration and apply it to the live bot.
#[utoipa::path(
    put,
    path = "/bot/presence",
    tag = "bot",
    security(("cookie_auth" = [])),
    request_body = BotPresence,
    responses(
        (status = 200, body = BotPresence),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 415, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    )
)]
pub async fn update_presence(
    State(state): State<AppState>,
    Json(mut presence): Json<BotPresence>,
) -> Response {
    if let Some(activity) = presence.activity.as_mut() {
        let text = activity.text.trim();

        if text.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("activity text is required")),
            )
                .into_response();
        }

        if text.chars().count() > MAX_ACTIVITY_TEXT {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "activity text must be 128 characters or fewer",
                )),
            )
                .into_response();
        }

        activity.text = text.to_owned();
    }

    match state.database().presence().save(&presence).await {
        Ok(saved) => {
            let _ = state.presence().send_replace(saved.clone());
            record_updated(&state, &saved).await;
            Json(saved).into_response()
        }
        Err(error) => storage_error(&error),
    }
}

async fn record_updated(state: &AppState, presence: &BotPresence) {
    let actor = dashboard_actor();
    let target = AuditEntity::new("bot", None, Some("Kira".to_owned()));
    let mut metadata = Map::new();
    metadata.insert("status".to_owned(), presence.status.as_str().into());

    if let Some(activity) = &presence.activity {
        metadata.insert("activity_kind".to_owned(), activity.kind.as_str().into());
        metadata.insert("activity_text".to_owned(), activity.text.clone().into());
    }

    record(
        state.database(),
        &NewAuditEvent::new(
            ACTION_PRESENCE_UPDATED,
            SOURCE_DASHBOARD,
            &actor,
            Some(&target),
            &metadata,
        ),
    )
    .await;
}
