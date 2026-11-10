use std::future::Future;
use std::net::SocketAddr;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use kira_config::ServerConfig;
use kira_protocol::HealthResponse;
use thiserror::Error;
use tokio::net::TcpListener;

/// Errors produced while binding or serving the HTTP server.
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("failed to bind server to {addr}")]
    Bind {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },

    #[error("server I/O error")]
    Io(#[from] std::io::Error),
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses((status = 200, body = HealthResponse))
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_upgrade))
}

pub async fn serve(
    config: &ServerConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServerError> {
    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|source| ServerError::Bind {
            addr: config.bind,
            source,
        })?;
    println!("Listening on http://{}", listener.local_addr()?);
    serve_with_listener(listener, shutdown).await
}

pub async fn serve_with_listener(
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServerError> {
    axum::serve(listener, router())
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
