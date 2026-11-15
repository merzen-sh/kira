//! Guild and channel handlers.

use std::time::Duration;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kira_core::messaging::OutgoingMessage;
use kira_protocol::{ChannelDetails, ErrorResponse, GuildDetails, SentMessage};

use crate::state::AppState;

/// How long a message POST waits for the bot to report a send result.
const MESSAGE_TIMEOUT: Duration = Duration::from_secs(15);

/// Channel kinds that can receive messages.
const SENDABLE_KINDS: [&str; 4] = ["text", "news", "public_thread", "private_thread"];

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
pub async fn get_guild(State(state): State<AppState>) -> Response {
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
pub async fn get_guild_channels(State(state): State<AppState>) -> Response {
    match state.guild_channels() {
        Some(channels) => Json(channels).into_response(),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new("guild data is not available")),
        )
            .into_response(),
    }
}

fn validate_message(payload: &serde_json::Value) -> Result<(), &'static str> {
    let object = payload.as_object().ok_or("message must be a JSON object")?;
    let content = object.get("content").and_then(|value| value.as_str());
    let has_embeds = object
        .get("embeds")
        .and_then(|value| value.as_array())
        .is_some_and(|embeds| !embeds.is_empty());

    if content.is_none_or(|content| content.trim().is_empty()) && !has_embeds {
        return Err("content or embeds is required");
    }

    if content.is_some_and(|content| content.chars().count() > 2000) {
        return Err("content must be 2000 characters or fewer");
    }

    Ok(())
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
    let Some(channels) = state.guild_channels() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new("guild data is not available")),
        )
            .into_response();
    };

    let Some(channel) = channels.iter().find(|channel| channel.id == channel_id) else {
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

    if let Err(error) = validate_message(&payload) {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new(error))).into_response();
    }

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
