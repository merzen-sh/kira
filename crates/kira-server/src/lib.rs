use std::future::Future;

use axum::{
    Json, Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
};
use kira_config::ServerConfig;
use kira_protocol::HealthResponse;
use tokio::net::TcpListener;

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
) -> kira_core::Result<()> {
    let listener = TcpListener::bind(config.bind).await?;
    println!("Listening on http://{}", listener.local_addr()?);
    serve_with_listener(listener, shutdown).await
}

pub async fn serve_with_listener(
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> kira_core::Result<()> {
    axum::serve(listener, router())
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

async fn ws_upgrade(upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(message)) = socket.recv().await {
        match message {
            Message::Text(text) => {
                if socket.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Message::Binary(bytes) => {
                if socket.send(Message::Binary(bytes)).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }
}
