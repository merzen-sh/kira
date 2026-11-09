use std::time::Duration;

use anyhow::{Result, anyhow};
use axum::{
    Json, Router,
    body::Body,
    extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade},
    http::{Request, StatusCode, header},
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use kira_cli::{AppState, router};
use kira_client::Client;
use kira_protocol::{ErrorResponse, HealthResponse};
use tokio::{net::TcpListener, task::JoinHandle, time::timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message};
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
    let app = Router::new().route("/health", get(|| async { Json(HealthResponse::ok()) }));
    spawn(app).await
}

async fn spawn_echo() -> Result<TestServer> {
    let app = Router::new().route("/ws", get(echo_upgrade));
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
    Ok(router(AppState {
        client,
        dev_server_url: None,
    })
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
        if let Err(error) = axum::serve(
            listener,
            router(AppState {
                client,
                dev_server_url: None,
            }),
        )
        .await
        {
            eprintln!("test server failed: {error}");
        }
    });

    let (mut socket, _response) = connect_async(format!("ws://{addr}/ws")).await?;

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
