use std::time::Duration;

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

async fn spawn(app: Router) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    TestServer { base_url, task }
}

async fn spawn_health() -> TestServer {
    let app = Router::new().route("/health", get(|| async { Json(HealthResponse::ok()) }));
    spawn(app).await
}

async fn spawn_echo() -> TestServer {
    let app = Router::new().route("/ws", get(echo_upgrade));
    spawn(app).await
}

async fn echo_upgrade(upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(echo)
}

async fn echo(mut socket: WebSocket) {
    while let Some(Ok(message)) = socket.recv().await {
        let is_close = matches!(message, AxumMessage::Close(_));
        if socket.send(message).await.is_err() || is_close {
            break;
        }
    }
}

async fn cli_get(client: Client, path: &str) -> axum::response::Response {
    router(AppState { client })
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn health_is_proxied() {
    let upstream = spawn_health().await;
    let res = cli_get(Client::new(upstream.base_url.clone()), "/api/health").await;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "application/json");

    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let health: HealthResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(health.status, "ok");
    assert_eq!(health.version, HealthResponse::ok().version);
}

#[tokio::test]
async fn unreachable_upstream_is_502() {
    let res = cli_get(Client::new("http://127.0.0.1:1"), "/api/health").await;

    assert_eq!(res.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "application/json");

    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let error: ErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert!(!error.error.is_empty());
}

#[tokio::test]
async fn websocket_frames_are_proxied() {
    let upstream = spawn_echo().await;
    let client = Client::new(upstream.base_url.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router(AppState { client }))
            .await
            .unwrap();
    });

    let (mut socket, _response) = connect_async(format!("ws://{addr}/ws")).await.unwrap();

    socket.send(Message::Text("hello".into())).await.unwrap();
    let echoed = timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("timed out waiting for an echoed text frame")
        .expect("the websocket stream ended")
        .expect("the echoed frame was an error");
    assert_eq!(echoed, Message::Text("hello".into()));

    socket
        .send(Message::Binary(vec![0x01, 0x02, 0x03].into()))
        .await
        .unwrap();
    let echoed = timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("timed out waiting for an echoed binary frame")
        .expect("the websocket stream ended")
        .expect("the echoed frame was an error");
    assert_eq!(echoed, Message::Binary(vec![0x01, 0x02, 0x03].into()));

    let _ = socket.close(None).await;
    server.abort();
}
