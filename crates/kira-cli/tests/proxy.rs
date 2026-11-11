//! API and WebSocket proxy integration tests.

use std::time::Duration;

use anyhow::{Result, anyhow};
use axum::body::Body;
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use axum::{Json, Router, routing};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use kira_cli::{AppState, router};
use kira_client::Client;
use kira_protocol::{ChannelDetails, ErrorResponse, GuildDetails, HealthResponse};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

struct TestServer {
    base_url: String,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn(app: Router) -> Result<TestServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let base_url = format!("http://{}", listener.local_addr()?);
    let task = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            eprintln!("test server failed: {error}");
        }
    });

    Ok(TestServer { base_url, task })
}

async fn spawn_health() -> Result<TestServer> {
    let app = Router::new().route(
        "/health",
        routing::get(|| async { Json(HealthResponse::ok()) }),
    );
    spawn(app).await
}

async fn spawn_guild() -> Result<TestServer> {
    let mut guild = GuildDetails::new("1", "Alpha");
    guild.member_count = 12;

    let app = Router::new().route(
        "/guild",
        routing::get(move || {
            let guild = guild.clone();
            async move { Json(guild) }
        }),
    );
    spawn(app).await
}

async fn spawn_guild_unavailable() -> Result<TestServer> {
    let app = Router::new().route(
        "/guild",
        routing::get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
    );
    spawn(app).await
}

async fn spawn_channels() -> Result<TestServer> {
    let channels = vec![ChannelDetails::new("10", "1", "general", "text")];

    let app = Router::new().route(
        "/guild/channels",
        routing::get(move || {
            let channels = channels.clone();
            async move { Json(channels) }
        }),
    );
    spawn(app).await
}

async fn spawn_channels_unavailable() -> Result<TestServer> {
    let app = Router::new().route(
        "/guild/channels",
        routing::get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
    );
    spawn(app).await
}

async fn spawn_echo() -> Result<TestServer> {
    let app = Router::new().route("/ws", routing::get(echo_upgrade));
    spawn(app).await
}

async fn echo_upgrade(upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(echo)
}

async fn echo(mut socket: WebSocket) {
    while let Some(result) = socket.recv().await {
        let message = match result {
            Ok(message) => message,
            Err(error) => {
                eprintln!("test websocket receive failed: {error}");
                break;
            }
        };
        let is_close = matches!(message, AxumMessage::Close(_));
        if let Err(error) = socket.send(message).await {
            eprintln!("test websocket send failed: {error}");
            break;
        }
        if is_close {
            break;
        }
    }
}

async fn cli_get(client: Client, path: &str) -> Result<axum::response::Response> {
    Ok(router(AppState::new(client, None))
        .oneshot(Request::builder().uri(path).body(Body::empty())?)
        .await?)
}

#[tokio::test]
async fn health_is_proxied() -> Result<()> {
    let upstream = spawn_health().await?;
    let res = cli_get(Client::new(upstream.base_url.clone()), "/api/health").await?;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "application/json");

    let bytes = res.into_body().collect().await?.to_bytes();
    let health: HealthResponse = serde_json::from_slice(&bytes)?;
    assert_eq!(health.status, "ok");
    assert_eq!(health.version, HealthResponse::ok().version);
    Ok(())
}

#[tokio::test]
async fn guild_is_proxied() -> Result<()> {
    let upstream = spawn_guild().await?;
    let res = cli_get(Client::new(upstream.base_url.clone()), "/api/guild").await?;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "application/json");

    let bytes = res.into_body().collect().await?.to_bytes();
    let guild: GuildDetails = serde_json::from_slice(&bytes)?;
    assert_eq!(guild.id, "1");
    assert_eq!(guild.member_count, 12);
    Ok(())
}

#[tokio::test]
async fn unavailable_guild_is_503() -> Result<()> {
    let upstream = spawn_guild_unavailable().await?;
    let res = cli_get(Client::new(upstream.base_url.clone()), "/api/guild").await?;

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

    let bytes = res.into_body().collect().await?.to_bytes();
    let error: ErrorResponse = serde_json::from_slice(&bytes)?;
    assert_eq!(error.error, "guild data is not available");
    Ok(())
}

#[tokio::test]
async fn channels_are_proxied() -> Result<()> {
    let upstream = spawn_channels().await?;
    let res = cli_get(
        Client::new(upstream.base_url.clone()),
        "/api/guild/channels",
    )
    .await?;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "application/json");

    let bytes = res.into_body().collect().await?.to_bytes();
    let channels: Vec<ChannelDetails> = serde_json::from_slice(&bytes)?;
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].name, "general");
    Ok(())
}

#[tokio::test]
async fn unavailable_channels_are_503() -> Result<()> {
    let upstream = spawn_channels_unavailable().await?;
    let res = cli_get(
        Client::new(upstream.base_url.clone()),
        "/api/guild/channels",
    )
    .await?;

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

    let bytes = res.into_body().collect().await?.to_bytes();
    let error: ErrorResponse = serde_json::from_slice(&bytes)?;
    assert_eq!(error.error, "guild data is not available");
    Ok(())
}

#[tokio::test]
async fn unreachable_upstream_is_502() -> Result<()> {
    let res = cli_get(Client::new("http://127.0.0.1:1"), "/api/health").await?;

    assert_eq!(res.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "application/json");

    let bytes = res.into_body().collect().await?.to_bytes();
    let error: ErrorResponse = serde_json::from_slice(&bytes)?;
    assert_eq!(error.error, "upstream service unavailable");
    Ok(())
}

#[tokio::test]
async fn websocket_frames_are_proxied() -> Result<()> {
    let upstream = spawn_echo().await?;
    let client = Client::new(upstream.base_url.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router(AppState::new(client, None))).await {
            eprintln!("test server failed: {error}");
        }
    });

    let (mut socket, _response) =
        tokio_tungstenite::connect_async(format!("ws://{addr}/ws")).await?;

    socket.send(Message::Text("hello".into())).await?;
    let echoed = timeout(Duration::from_secs(5), socket.next())
        .await?
        .ok_or_else(|| anyhow!("the websocket stream ended before an echoed text frame"))??;
    assert_eq!(echoed, Message::Text("hello".into()));

    socket
        .send(Message::Binary(vec![0x01, 0x02, 0x03].into()))
        .await?;
    let echoed = timeout(Duration::from_secs(5), socket.next())
        .await?
        .ok_or_else(|| anyhow!("the websocket stream ended before an echoed binary frame"))??;
    assert_eq!(echoed, Message::Binary(vec![0x01, 0x02, 0x03].into()));

    socket.close(None).await?;
    server.abort();
    Ok(())
}
