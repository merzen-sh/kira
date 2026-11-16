//! Guild and channel handlers.

use std::time::Duration;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kira_core::messaging::OutgoingMessage;
use kira_protocol::{
    AuditEntity, ChannelDetails, ErrorResponse, GuildDetails, SOURCE_DASHBOARD, SentMessage,
};
use kira_store::audit::NewAuditEvent;
use serde_json::Map;

use crate::handlers::audit::{dashboard_actor, record};
use crate::state::AppState;

/// Action recorded when a dashboard admin publishes a message.
const ACTION_MESSAGE_SENT: &str = "message.sent";

/// Facts about a validated message payload, reused for audit metadata.
struct MessageSummary {
    content_length: usize,
    has_embeds: bool,
}

/// How long a message POST waits for the bot to report a send result.
const MESSAGE_TIMEOUT: Duration = Duration::from_secs(15);

/// Channel kinds that can receive messages.
const SENDABLE_KINDS: [&str; 4] = ["text", "news", "public_thread", "private_thread"];

/// Discord's maximum message content length, in characters.
const MAX_CONTENT_LENGTH: usize = 2000;

/// Return the configured Discord server's details.
#[utoipa::path(
    get,
    path = "/guild",
    tag = "guild",
    security(("cookie_auth" = [])),
    responses(
        (status = 200, body = GuildDetails),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 503, body = ErrorResponse)
    )
)]
pub async fn guild(State(state): State<AppState>) -> Response {
    match state.guild() {
        Some(guild) => Json(guild).into_response(),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new("guild data is not available")),
        )
            .into_response(),
    }
}

/// Return the configured Discord server's channels.
#[utoipa::path(
    get,
    path = "/guild/channels",
    tag = "guild",
    security(("cookie_auth" = [])),
    responses(
        (status = 200, body = [ChannelDetails]),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 503, body = ErrorResponse)
    )
)]
pub async fn guild_channels(State(state): State<AppState>) -> Response {
    match state.guild_channels() {
        Some(channels) => Json(channels).into_response(),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new("guild data is not available")),
        )
            .into_response(),
    }
}

fn validate_message(payload: &serde_json::Value) -> Result<MessageSummary, &'static str> {
    let object = payload.as_object().ok_or("message must be a JSON object")?;
    let content = object.get("content").and_then(|value| value.as_str());
    let content_length = content.map_or(0, |content| content.chars().count());
    let has_content = content.is_some_and(|content| !content.trim().is_empty());
    let has_embeds = object
        .get("embeds")
        .and_then(|value| value.as_array())
        .is_some_and(|embeds| !embeds.is_empty());

    if !has_content && !has_embeds {
        return Err("content or embeds is required");
    }

    if content_length > MAX_CONTENT_LENGTH {
        return Err("content must be 2000 characters or fewer");
    }

    Ok(MessageSummary {
        content_length,
        has_embeds,
    })
}

/// Post a message to one of the configured Discord server's channels.
#[utoipa::path(
    post,
    path = "/guild/channels/{channel_id}/messages",
    tag = "guild",
    security(("cookie_auth" = [])),
    params(("channel_id" = String, Path, description = "Discord channel ID")),
    request_body = serde_json::Value,
    responses(
        (status = 201, body = SentMessage),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 415, body = ErrorResponse),
        (status = 422, body = ErrorResponse),
        (status = 502, body = ErrorResponse),
        (status = 503, body = ErrorResponse),
        (status = 504, body = ErrorResponse)
    )
)]
pub async fn send_guild_message(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    if !state.has_guild() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new("guild data is not available")),
        )
            .into_response();
    }

    let Some(channel) = state.channel(&channel_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("channel is not available")),
        )
            .into_response();
    };

    if !SENDABLE_KINDS.contains(&channel.kind.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("channel cannot receive messages")),
        )
            .into_response();
    }

    let summary = match validate_message(&payload) {
        Ok(summary) => summary,
        Err(error) => {
            return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new(error))).into_response();
        }
    };

    let (message, response) = OutgoingMessage::new(channel_id.clone(), payload);

    if state.messages().send(message).await.is_err() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new("message delivery is not available")),
        )
            .into_response();
    }

    match tokio::time::timeout(MESSAGE_TIMEOUT, response).await {
        Ok(Ok(Ok(id))) => {
            let actor = dashboard_actor();
            let target = AuditEntity::new(
                "channel",
                Some(channel.id.clone()),
                Some(channel.name.clone()),
            );
            let mut metadata = Map::new();
            metadata.insert("message_id".to_owned(), id.clone().into());
            metadata.insert(
                "content_length".to_owned(),
                u64::try_from(summary.content_length)
                    .unwrap_or_default()
                    .into(),
            );
            metadata.insert("has_embeds".to_owned(), summary.has_embeds.into());
            record(
                state.database(),
                &NewAuditEvent::new(
                    ACTION_MESSAGE_SENT,
                    SOURCE_DASHBOARD,
                    &actor,
                    Some(&target),
                    &metadata,
                ),
            )
            .await;

            (StatusCode::CREATED, Json(SentMessage::new(id, channel_id))).into_response()
        }
        Ok(Ok(Err(error))) => {
            (StatusCode::BAD_GATEWAY, Json(ErrorResponse::new(error))).into_response()
        }
        Ok(Err(_)) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new("message delivery failed")),
        )
            .into_response(),
        Err(_) => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(ErrorResponse::new("message delivery timed out")),
        )
            .into_response(),
    }
}
