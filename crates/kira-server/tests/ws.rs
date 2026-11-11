//! WebSocket echo integration tests.

use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use kira_server::{AppState, ServerError, serve_with_listener};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

struct TestServer {
    addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), ServerError>>,
}

async fn spawn_server() -> Result<TestServer, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (shutdown, shutdown_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        serve_with_listener(listener, AppState::default(), async {
            if let Err(error) = shutdown_rx.await {
                eprintln!("test shutdown channel closed: {error}");
            }
        })
        .await
    });

    Ok(TestServer {
        addr,
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

#[tokio::test]
async fn ws_echoes_text_frames() -> Result<(), Box<dyn Error>> {
    let server = spawn_server().await?;
    let (mut socket, _response) =
        tokio_tungstenite::connect_async(format!("ws://{}/ws", server.addr)).await?;
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
async fn ws_echoes_binary_frames() -> Result<(), Box<dyn Error>> {
    let server = spawn_server().await?;
    let (mut socket, _response) =
        tokio_tungstenite::connect_async(format!("ws://{}/ws", server.addr)).await?;
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
