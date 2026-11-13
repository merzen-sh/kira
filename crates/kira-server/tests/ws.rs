//! WebSocket authentication and echo integration tests.

mod common;

use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

use axum::http::{HeaderValue, StatusCode, header};
use futures_util::{SinkExt, StreamExt};
use kira_server::{ServerError, serve_with_listener};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

const ALLOWED_ORIGIN: &str = "http://127.0.0.1:3000";

struct TestServer {
    addr: SocketAddr,
    cookie: String,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), ServerError>>,
}

async fn spawn_server() -> Result<TestServer, Box<dyn Error>> {
    let state = common::empty_state();
    let cookie = common::login_cookie(&state).await?;

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (shutdown, shutdown_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        serve_with_listener(listener, state, async {
            if let Err(error) = shutdown_rx.await {
                eprintln!("test shutdown channel closed: {error}");
            }
        })
        .await
    });

    Ok(TestServer {
        addr,
        cookie,
        shutdown,
        task,
    })
}

async fn stop(server: TestServer) -> Result<(), Box<dyn Error>> {
    server
        .shutdown
        .send(())
        .map_err(|()| "shutdown receiver dropped")?;
    let result = timeout(Duration::from_secs(5), server.task).await??;
    result?;
    Ok(())
}

fn request(
    addr: SocketAddr,
    cookie: Option<&str>,
    origin: Option<&str>,
) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request, Box<dyn Error>> {
    let mut request = format!("ws://{addr}/ws").into_client_request()?;

    if let Some(cookie) = cookie {
        request
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(cookie)?);
    }
    if let Some(origin) = origin {
        request
            .headers_mut()
            .insert(header::ORIGIN, HeaderValue::from_str(origin)?);
    }

    Ok(request)
}

async fn assert_rejected(
    addr: SocketAddr,
    cookie: Option<&str>,
    origin: Option<&str>,
    expected: StatusCode,
) -> Result<(), Box<dyn Error>> {
    let error = tokio_tungstenite::connect_async(request(addr, cookie, origin)?)
        .await
        .err()
        .ok_or("the websocket handshake unexpectedly succeeded")?;

    match error {
        WsError::Http(response) => assert_eq!(response.status(), expected),
        other => return Err(format!("unexpected websocket error: {other}").into()),
    }

    Ok(())
}

#[tokio::test]
async fn ws_requires_a_session_cookie() -> Result<(), Box<dyn Error>> {
    let server = spawn_server().await?;
    assert_rejected(server.addr, None, None, StatusCode::UNAUTHORIZED).await?;
    assert_rejected(
        server.addr,
        Some("kira_session=v1.0.invalid"),
        None,
        StatusCode::UNAUTHORIZED,
    )
    .await?;
    stop(server).await
}

#[tokio::test]
async fn ws_rejects_disallowed_origins() -> Result<(), Box<dyn Error>> {
    let server = spawn_server().await?;
    assert_rejected(
        server.addr,
        Some(&server.cookie),
        Some("http://evil.example"),
        StatusCode::FORBIDDEN,
    )
    .await?;
    stop(server).await
}

#[tokio::test]
async fn ws_echoes_text_frames_for_authenticated_clients() -> Result<(), Box<dyn Error>> {
    let server = spawn_server().await?;
    let (mut socket, _response) = tokio_tungstenite::connect_async(request(
        server.addr,
        Some(&server.cookie),
        Some(ALLOWED_ORIGIN),
    )?)
    .await?;

    socket.send(Message::Text("hello".into())).await?;
    let echoed = timeout(Duration::from_secs(5), socket.next())
        .await?
        .ok_or("the websocket stream ended")??;
    assert_eq!(echoed, Message::Text("hello".into()));

    socket.close(None).await?;
    stop(server).await?;
    Ok(())
}

#[tokio::test]
async fn ws_echoes_binary_frames_for_authenticated_clients() -> Result<(), Box<dyn Error>> {
    let server = spawn_server().await?;
    let (mut socket, _response) =
        tokio_tungstenite::connect_async(request(server.addr, Some(&server.cookie), None)?).await?;

    let payload = vec![0x01, 0x02, 0x03];
    socket.send(Message::Binary(payload.clone().into())).await?;
    let echoed = timeout(Duration::from_secs(5), socket.next())
        .await?
        .ok_or("the websocket stream ended")??;
    assert_eq!(echoed, Message::Binary(payload.into()));

    socket.close(None).await?;
    stop(server).await?;
    Ok(())
}
