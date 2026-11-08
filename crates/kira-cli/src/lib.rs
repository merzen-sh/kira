//! Local dashboard server that proxies API and WebSocket traffic to a remote
//! Kira server.

pub mod assets;

mod proxy;

use axum::Router;

/// Shared state for the local CLI server.
#[derive(Clone)]
pub struct AppState {
    pub client: kira_client::Client,
}

/// Build the local router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", axum::routing::get(proxy::health))
        .route("/ws", axum::routing::get(proxy::ws_upgrade))
        .fallback(assets::fallback)
        .with_state(state)
}
