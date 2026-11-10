//! WebSocket client integration tests.

use std::error::Error;
use std::time::Duration;

use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::{Router, routing};
use futures_util::{SinkExt, StreamExt};
use kira_client::Message;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::timeout;

struct TestServer {
    base_url: String,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn_echo() -> Result<(kira_client::Client, TestServer), Box<dyn Error>> {
    let app = Router::new().route("/ws", routing::get(upgrade));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let base_url = format!("http://{}", listener.local_addr()?);
    let task = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            eprintln!("test server failed: {error}");
        }
    });
    let server = TestServer { base_url, task };

    Ok((kira_client::Client::new(server.base_url.clone()), server))
}

async fn upgrade(upgrade: WebSocketUpgrade) -> Response {
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
        match message {
            AxumMessage::Text(text) => {
                if let Err(error) = socket.send(AxumMessage::Text(text)).await {
                    eprintln!("test websocket send failed: {error}");
                    break;
                }
            }
            AxumMessage::Close(_) => break,
            _ => {}
        }
    }
}

#[tokio::test]
async fn connect_ws_echoes_text_frames() -> Result<(), Box<dyn Error>> {
    let (client, _server) = spawn_echo().await?;
    let mut stream = client.connect_ws("/ws").await?;

    stream.send(Message::Text("hello".into())).await?;

    let echoed = timeout(Duration::from_secs(5), stream.next())
        .await?
        .ok_or("the websocket stream ended")??;

    assert_eq!(echoed, Message::Text("hello".into()));
    Ok(())
}
