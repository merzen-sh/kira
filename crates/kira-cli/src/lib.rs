//! Local dashboard host that serves the built Vite application.

pub mod assets;

#[cfg(feature = "dev-proxy")]
mod dev_proxy;

use axum::Router;

/// Shared state for the local CLI server.
#[derive(Clone)]
#[non_exhaustive]
pub struct AppState {
    /// When set, non-API requests are proxied to this Vite dev server instead of
    /// serving the embedded dashboard assets.
    pub dev_server_url: Option<String>,
}

impl AppState {
    /// Build the shared state for the local dashboard server.
    #[must_use]
    pub fn new(dev_server_url: Option<String>) -> Self {
        Self { dev_server_url }
    }
}

/// Build the local router.
pub fn router(state: AppState) -> Router {
    Router::new().fallback(assets::fallback).with_state(state)
}
