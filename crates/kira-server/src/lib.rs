//! HTTP and WebSocket server for the Kira API.

#![expect(
    clippy::exhaustive_structs,
    reason = "utoipa::path generates an internal exhaustive route struct"
)]

use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Extension, Path, Query, Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
#[cfg(debug_assertions)]
use axum::response::Html;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing};
use kira_config::ServerConfig;
use kira_core::messaging::{MessageSender, OutgoingMessage};
use kira_protocol::{
    ChannelDetails, ErrorResponse, GuildDetails, HealthResponse, LoginRequest, Report, ReportList,
    SentMessage, UpdateReport,
};
use kira_store::Store;
use serde::Deserialize;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tower_http::cors::{AllowOrigin, CorsLayer};
#[cfg(debug_assertions)]
use utoipa::OpenApi;

mod auth;

pub use auth::{Auth, AuthConfig, COOKIE_NAME, LoginError, SESSION_TTL};

/// How long a message POST waits for the bot to report a send result.
const MESSAGE_TIMEOUT: Duration = Duration::from_secs(15);

/// Channel kinds that can receive messages.
const SENDABLE_KINDS: [&str; 4] = ["text", "news", "public_thread", "private_thread"];

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

    /// A non-loopback bind was requested without HTTPS, secure cookies, and
    /// an origin allowlist.
    #[error(
        "refusing to bind {addr} without KIRA_HTTPS=true, KIRA_COOKIE_SECURE=true, and KIRA_ALLOWED_ORIGINS"
    )]
    InsecureBind {
        /// Address the server refused to bind.
        addr: SocketAddr,
    },

    /// The dashboard binds beyond loopback without an origin allowlist.
    #[error("KIRA_ALLOWED_ORIGINS is required when the dashboard binds {addr}")]
    OriginsRequired {
        /// Address the dashboard host refused to derive origins for.
        addr: SocketAddr,
    },

    /// The configured origin allowlist is empty or malformed.
    #[error("KIRA_ALLOWED_ORIGINS must be a comma-separated list of http(s) origins")]
    InvalidOrigin,

    /// `SameSite=None` was requested without the `Secure` attribute.
    #[error("KIRA_COOKIE_SAMESITE=none requires KIRA_COOKIE_SECURE=true")]
    InsecureCookie,

    /// A random secret could not be generated.
    #[error("failed to generate a random secret")]
    Random,

    /// The server failed while accepting or serving connections.
    #[error("server I/O error")]
    Io(#[from] std::io::Error),
}

/// Shared state for the HTTP server.
#[derive(Clone)]
pub struct AppState {
    guild_id: String,
    guilds: watch::Receiver<Vec<GuildDetails>>,
    channels: watch::Receiver<Vec<ChannelDetails>>,
    reports: Store,
    messages: MessageSender,
    auth: Auth,
}

impl AppState {
    /// Build state from the configured guild ID, the bot's cached state,
    /// report storage, and the authentication settings.
    #[must_use]
    pub fn new(
        guild_id: u64,
        guilds: watch::Receiver<Vec<GuildDetails>>,
        channels: watch::Receiver<Vec<ChannelDetails>>,
        reports: Store,
        messages: MessageSender,
        auth: Auth,
    ) -> Self {
        Self {
            guild_id: guild_id.to_string(),
            guilds,
            channels,
            reports,
            messages,
            auth,
        }
    }

    fn guild(&self) -> Option<GuildDetails> {
        self.guilds
            .borrow()
            .iter()
            .find(|guild| guild.id == self.guild_id)
            .cloned()
    }

    fn guild_channels(&self) -> Option<Vec<ChannelDetails>> {
        self.guild()?;

        Some(
            self.channels
                .borrow()
                .iter()
                .filter(|channel| channel.guild_id == self.guild_id)
                .cloned()
                .collect(),
        )
    }
}

/// Peer address captured from the TCP connection, when available.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct PeerAddress(Option<IpAddr>);

/// Record the connection peer for the login rate limiter.
async fn record_peer(mut request: Request, next: Next) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| connect.0.ip());
    request.extensions_mut().insert(PeerAddress(peer));
    next.run(request).await
}

/// Reject requests whose `Origin` header is not allowlisted.
async fn require_origin(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());

    if origin.is_some_and(|origin| !state.auth.origin_allowed(origin)) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("origin is not allowed")),
        )
            .into_response();
    }

    next.run(request).await
}

/// Reject requests without a valid session cookie.
async fn require_session(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let authorized = state
        .auth
        .token_from_headers(request.headers())
        .is_some_and(|token| state.auth.verify_token(&token));

    if authorized {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("authentication required")),
        )
            .into_response()
    }
}

fn cors_layer(state: &AppState) -> CorsLayer {
    let origins = state
        .auth
        .allowed_origins()
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
        .allow_origin(AllowOrigin::list(origins))
        .max_age(Duration::from_secs(600))
}

/// Return the service health payload.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, body = HealthResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
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
    let mut response = match state.auth.login(&body.pin, peer.0) {
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
            (header::SET_COOKIE, state.auth.clear_cookie()),
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

    if state.messages.send(message).await.is_err() {
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

/// Default number of reports returned by `GET /reports`.
const DEFAULT_PAGE_LIMIT: u32 = 20;

/// Largest page size accepted by `GET /reports`.
const MAX_PAGE_LIMIT: u32 = 100;

/// Query parameters for `GET /reports`.
#[derive(Debug, Deserialize)]
pub struct ReportListQuery {
    page: Option<u32>,
    page_limit: Option<u32>,
    search: Option<String>,
}

fn storage_error(error: &kira_store::StoreError) -> Response {
    tracing::error!("report storage failed: {error}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("report storage failed")),
    )
        .into_response()
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
    let page = query.page.unwrap_or(1).max(1);
    let page_limit = query
        .page_limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .clamp(1, MAX_PAGE_LIMIT);
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match state.reports.list_reports(page, page_limit, search).await {
        Ok(reports) => Json(reports).into_response(),
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
    match state.reports.set_resolved(id, body.resolved).await {
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
    match state.reports.delete_report(id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("report was not found")),
        )
            .into_response(),
        Err(error) => storage_error(&error),
    }
}

/// OpenAPI document assembled from the server's route annotations.
#[derive(utoipa::OpenApi)]
#[non_exhaustive]
#[openapi(
    info(title = "Kira API", version = env!("CARGO_PKG_VERSION"), description = "Kira service API"),
    tags(
        (name = "auth", description = "Dashboard session authentication"),
        (name = "health", description = "Service health"),
        (name = "guild", description = "Discord server and channels"),
        (name = "reports", description = "User-submitted bug reports")
    ),
    paths(
        health,
        login,
        logout,
        session,
        get_guild,
        get_guild_channels,
        send_guild_message,
        list_reports,
        update_report,
        delete_report
    ),
    components(schemas(
            kira_protocol::HealthResponse,
            kira_protocol::GuildDetails,
            kira_protocol::ChannelDetails,
            kira_protocol::SentMessage,
            kira_protocol::LoginRequest,
            kira_protocol::Report,
            kira_protocol::ReportList,
            kira_protocol::UpdateReport,
            kira_protocol::ErrorResponse
        )),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};

        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "cookie_auth",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(COOKIE_NAME))),
            );
        }
    }
}

/// Build the HTTP router for the Kira API.
pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/auth/session", routing::get(session))
        .route("/guild", routing::get(get_guild))
        .route("/guild/channels", routing::get(get_guild_channels))
        .route(
            "/guild/channels/{channel_id}/messages",
            routing::post(send_guild_message),
        )
        .route("/reports", routing::get(list_reports))
        .route(
            "/reports/{id}",
            routing::patch(update_report).delete(delete_report),
        )
        .route("/ws", routing::get(ws_upgrade))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_session,
        ));

    let router = Router::new()
        .route("/health", routing::get(health))
        .route("/auth/login", routing::post(login))
        .route("/auth/logout", routing::post(logout))
        .merge(protected)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_origin,
        ))
        .layer(middleware::from_fn(record_peer))
        .layer(cors_layer(&state))
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
    reports: Store,
    messages: MessageSender,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServerError> {
    ensure_secure_bind(
        config.bind,
        config.https,
        config.cookie_secure,
        config.allowed_origins.is_some(),
    )?;

    let auth = Auth::new(AuthConfig::from_server_config(config)?)?;
    if let Some(pin) = auth.generated_pin() {
        tracing::warn!("KIRA_PASSWORD is not set; generated dashboard PIN: {pin}");
    }

    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|source| ServerError::Bind {
            addr: config.bind,
            source,
        })?;
    tracing::info!("Listening on http://{}", listener.local_addr()?);
    serve_with_listener(
        listener,
        AppState::new(config.guild_id, guilds, channels, reports, messages, auth),
        shutdown,
    )
    .await
}

fn ensure_secure_bind(
    bind: SocketAddr,
    https: bool,
    cookie_secure: bool,
    has_allowed_origins: bool,
) -> Result<(), ServerError> {
    if bind.ip().is_loopback() || (https && cookie_secure && has_allowed_origins) {
        return Ok(());
    }

    Err(ServerError::InsecureBind { addr: bind })
}

/// Serve on an already-bound listener until `shutdown` resolves.
pub async fn serve_with_listener(
    listener: TcpListener,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServerError> {
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
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
                tracing::error!("websocket receive failed: {error}");
                break;
            }
        };

        match message {
            Message::Text(text) => {
                if let Err(error) = socket.send(Message::Text(text)).await {
                    tracing::error!("websocket send failed: {error}");
                    break;
                }
            }
            Message::Binary(bytes) => {
                if let Err(error) = socket.send(Message::Binary(bytes)).await {
                    tracing::error!("websocket send failed: {error}");
                    break;
                }
            }
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_bind_needs_no_origin_configuration() {
        assert!(
            ensure_secure_bind(
                SocketAddr::from(([127, 0, 0, 1], 8080)),
                false,
                false,
                false
            )
            .is_ok()
        );
    }

    #[test]
    fn public_bind_requires_https_secure_cookies_and_an_allowlist() {
        let public = SocketAddr::from(([0, 0, 0, 0], 8080));
        assert!(matches!(
            ensure_secure_bind(public, false, false, false),
            Err(ServerError::InsecureBind { .. })
        ));
        assert!(matches!(
            ensure_secure_bind(public, true, false, true),
            Err(ServerError::InsecureBind { .. })
        ));
        assert!(matches!(
            ensure_secure_bind(public, false, true, true),
            Err(ServerError::InsecureBind { .. })
        ));
        assert!(matches!(
            ensure_secure_bind(public, true, true, false),
            Err(ServerError::InsecureBind { .. })
        ));
        assert!(ensure_secure_bind(public, true, true, true).is_ok());
    }
}
