//! Local dashboard host.

use anyhow::Context;
use kira_cli::{AppState, router};
use kira_config::CliConfig;
use tokio::net::TcpListener;

const VITE_DEV_SERVER_URL: &str = "http://127.0.0.1:5173";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    kira_core::logging::init();

    let config = CliConfig::from_env().context("loading CLI configuration")?;
    let dev_server_url = cfg!(debug_assertions).then(|| VITE_DEV_SERVER_URL.to_owned());

    let listener = TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("binding {}", config.bind))?;
    let address = listener
        .local_addr()
        .context("reading the local listener address")?;
    tracing::info!("Listening on http://{address}");

    axum::serve(listener, router(AppState::new(dev_server_url)))
        .with_graceful_shutdown(async {
            if let Err(error) = kira_core::shutdown::wait_for_signal().await {
                tracing::error!("failed to wait for shutdown signal: {error}");
            }
        })
        .await
        .context("serving the local dashboard")?;

    Ok(())
}
