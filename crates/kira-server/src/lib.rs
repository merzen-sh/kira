//! HTTP and WebSocket server for the Kira API.

#![expect(
    clippy::exhaustive_structs,
    reason = "utoipa::path generates an internal exhaustive route struct"
)]

use std::future::Future;
use std::net::SocketAddr;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
#[cfg(debug_assertions)]
use axum::response::Html;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing};
use kira_config::ServerConfig;
use kira_core::messaging::{MessageSender, OutgoingMessage};
use kira_protocol::{ChannelDetails, ErrorResponse, GuildDetails, HealthResponse, SentMessage};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};
#[cfg(debug_assertions)]
use utoipa::OpenApi;

/// Errors produced while binding or serving the HTTP server.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServerError {
    /// The listener could not bind to the configured address.
    #[error("failed to bind server to {addr}")]
    Bind {
        /// Address the server tried to bind.
        addr: SocketAddr,
        /// Underlying bind failure.
        #[source]
        source: std::io::Error,
    },

    /// The server failed while accepting or serving connections.
    #[error("server I/O error")]
    Io(#[from] std::io::Error),
}

/// Shared state for the HTTP server.
#[derive(Clone)]
pub struct AppState {
    guild_id: Option<String>,
    guilds: watch::Receiver<Vec<GuildDetails>>,
    channels: watch::Receiver<Vec<ChannelDetails>>,
    messages: MessageSender,
}

impl AppState {
    /// Build state from the configured guild ID and the bot's cached state.
    #[must_use]
    pub fn new(
        guild_id: Option<u64>,
        guilds: watch::Receiver<Vec<GuildDetails>>,
        channels: watch::Receiver<Vec<ChannelDetails>>,
        messages: MessageSender,
    ) -> Self {
        Self {
            guild_id: guild_id.map(|id| id.to_string()),
            guilds,
            channels,
            messages,
        }
    }

    fn guild(&self) -> Option<GuildDetails> {
        let target = self.guild_id.as_deref()?;

        self.guilds
            .borrow()
            .iter()
            .find(|guild| guild.id == target)
            .cloned()
    }

    fn guild_channels(&self) -> Option<Vec<ChannelDetails>> {
        let target = self.guild_id.as_deref()?;
        self.guild()?;

        Some(
            self.channels
                .borrow()
                .iter()
                .filter(|channel| channel.guild_id == target)
                .cloned()
                .collect(),
        )
    }
}

impl Default for AppState {
    fn default() -> Self {
        let (_guild_sender, guilds) = watch::channel(Vec::new());
        let (_channel_sender, channels) = watch::channel(Vec::new());
        let (messages, _receiver) = mpsc::channel(1);
        Self::new(None, guilds, channels, messages)
    }
}

/// Return the service health payload.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses((status = 200, body = HealthResponse))
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}

/// Return the configured Discord server's details.
#[utoipa::path(
    get,
    path = "/guild",
    tag = "guild",
    responses(
        (status = 200, body = GuildDetails),
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
    responses(
        (status = 200, body = [ChannelDetails]),
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
    params(("channel_id" = String, Path, description = "Discord channel ID")),
    request_body = serde_json::Value,
    responses(
        (status = 201, body = SentMessage),
        (status = 400, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 502, body = ErrorResponse),
        (status = 503, body = ErrorResponse)
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

    if !channels.iter().any(|channel| channel.id == channel_id) {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("channel is not available")),
        )
            .into_response();
    }

    if let Err(error) = validate_message(&payload) {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new(error))).into_response();
    }

    let (message, response) = OutgoingMessage::new(channel_id.clone(), payload);

    if state.messages.send(message).await.is_err() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new("message delivery is not available")),
        )
            .into_response();
    }

    match response.await {
        Ok(Ok(id)) => (StatusCode::CREATED, Json(SentMessage::new(id, channel_id))).into_response(),
        Ok(Err(error)) => {
            (StatusCode::BAD_GATEWAY, Json(ErrorResponse::new(error))).into_response()
        }
        Err(_) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new("message delivery failed")),
        )
            .into_response(),
    }
}

/// OpenAPI document assembled from the server's route annotations.
#[derive(utoipa::OpenApi)]
#[non_exhaustive]
#[openapi(
    info(title = "Kira API", version = env!("CARGO_PKG_VERSION"), description = "Kira service API"),
    tags(
        (name = "health", description = "Service health"),
        (name = "guild", description = "Discord server and channels")
    ),
    paths(health, get_guild, get_guild_channels, send_guild_message),
    components(schemas(
        kira_protocol::HealthResponse,
        kira_protocol::GuildDetails,
        kira_protocol::ChannelDetails,
        kira_protocol::SentMessage,
        kira_protocol::ErrorResponse
    ))
)]
pub struct ApiDoc;

/// Build the HTTP router for the Kira API.
pub fn router(state: AppState) -> Router {
    let router = Router::new()
        .route("/health", routing::get(health))
        .route("/guild", routing::get(get_guild))
        .route("/guild/channels", routing::get(get_guild_channels))
        .route(
            "/guild/channels/{channel_id}/messages",
            routing::post(send_guild_message),
        )
        .route("/ws", routing::get(ws_upgrade))
        .with_state(state);

    #[cfg(debug_assertions)]
    let router = router
        .route("/docs", routing::get(scalar_ui))
        .route("/openapi.json", routing::get(openapi_json));

    router
}

#[cfg(debug_assertions)]
const SCALAR_UI: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Kira API Reference</title>
  </head>
  <body>
    <script id="api-reference" data-url="/openapi.json"></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
  </body>
</html>
"#;

#[cfg(debug_assertions)]
async fn scalar_ui() -> Html<&'static str> {
    Html(SCALAR_UI)
}

#[cfg(debug_assertions)]
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// Bind to the configured address and serve until `shutdown` resolves.
pub async fn serve(
    config: &ServerConfig,
    guilds: watch::Receiver<Vec<GuildDetails>>,
    channels: watch::Receiver<Vec<ChannelDetails>>,
    messages: MessageSender,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServerError> {
    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|source| ServerError::Bind {
            addr: config.bind,
            source,
        })?;
    println!("Listening on http://{}", listener.local_addr()?);
    serve_with_listener(
        listener,
        AppState::new(config.guild_id, guilds, channels, messages),
        shutdown,
    )
    .await
}

/// Serve on an already-bound listener until `shutdown` resolves.
pub async fn serve_with_listener(
    listener: TcpListener,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServerError> {
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

async fn ws_upgrade(upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(result) = socket.recv().await {
        let message = match result {
            Ok(message) => message,
            Err(error) => {
                eprintln!("websocket receive failed: {error}");
                break;
            }
        };

        match message {
            Message::Text(text) => {
                if let Err(error) = socket.send(Message::Text(text)).await {
                    eprintln!("websocket send failed: {error}");
                    break;
                }
            }
            Message::Binary(bytes) => {
                if let Err(error) = socket.send(Message::Binary(bytes)).await {
                    eprintln!("websocket send failed: {error}");
                    break;
                }
            }
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }
}
