//! HTTP and WebSocket server for the Kira API.

#![expect(
    clippy::exhaustive_structs,
    reason = "utoipa::path generates an internal exhaustive route struct"
)]

use std::future::Future;
use std::net::SocketAddr;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::{Json, Router, routing};
use kira_config::ServerConfig;
use kira_protocol::HealthResponse;
use thiserror::Error;
use tokio::net::TcpListener;

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

    /// The server failed while accepting or serving connections.
    #[error("server I/O error")]
    Io(#[from] std::io::Error),
}

/// Return the service health payload.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses((status = 200, body = HealthResponse))
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}

/// Build the HTTP router for the Kira API.
pub fn router() -> Router {
    Router::new()
        .route("/health", routing::get(health))
        .route("/ws", routing::get(ws_upgrade))
}

/// Bind to the configured address and serve until `shutdown` resolves.
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

/// Serve on an already-bound listener until `shutdown` resolves.
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
