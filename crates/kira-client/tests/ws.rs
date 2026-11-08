use std::time::Duration;

use axum::{
    Router,
    extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use kira_client::Message;
use tokio::{net::TcpListener, task::JoinHandle, time::timeout};

struct TestServer {
    base_url: String,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn_echo() -> (kira_client::Client, TestServer) {
    let app = Router::new().route("/ws", get(upgrade));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let server = TestServer { base_url, task };

    (kira_client::Client::new(server.base_url.clone()), server)
}

async fn upgrade(upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(echo)
}

async fn echo(mut socket: WebSocket) {
    while let Some(Ok(message)) = socket.recv().await {
        match message {
            AxumMessage::Text(text) => {
                if socket.send(AxumMessage::Text(text)).await.is_err() {
                    break;
                }
            }
            AxumMessage::Close(_) => break,
            _ => {}
        }
    }
}

#[tokio::test]
async fn connect_ws_echoes_text_frames() {
    let (client, _server) = spawn_echo().await;
    let mut stream = client.connect_ws("/ws").await.unwrap();

    stream.send(Message::Text("hello".into())).await.unwrap();

    let echoed = timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("timed out waiting for an echoed frame")
        .expect("the websocket stream ended")
        .expect("the echoed frame was an error");

    assert_eq!(echoed, Message::Text("hello".into()));
}
