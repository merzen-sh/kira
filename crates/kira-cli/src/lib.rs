//! Local dashboard server that proxies API and WebSocket traffic to a remote
//! Kira server.

pub mod assets;

mod dev_proxy;
mod proxy;

use axum::Router;

/// Shared state for the local CLI server.
#[derive(Clone)]
#[non_exhaustive]
pub struct AppState {
    /// Client used to reach the remote Kira server.
    pub client: kira_client::Client,
    /// When set, non-API requests are proxied to this Vite dev server instead of
    /// serving the embedded dashboard assets.
    pub dev_server_url: Option<String>,
}

impl AppState {
    /// Build the shared state for the local dashboard server.
    #[must_use]
    pub fn new(client: kira_client::Client, dev_server_url: Option<String>) -> Self {
        Self {
            client,
            dev_server_url,
        }
    }
}

/// Build the local router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", axum::routing::get(proxy::health))
        .route("/api/guild", axum::routing::get(proxy::guild))
        .route("/api/guild/channels", axum::routing::get(proxy::channels))
        .route(
            "/api/guild/channels/{channel_id}/messages",
            axum::routing::post(proxy::send_message),
        )
        .route("/ws", axum::routing::get(proxy::ws_upgrade))
        .fallback(assets::fallback)
        .with_state(state)
}
