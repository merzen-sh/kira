//! WebSocket route and echo loop.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;

pub async fn ws_upgrade(upgrade: WebSocketUpgrade) -> Response {
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
