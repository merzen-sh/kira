use anyhow::Context;
use kira_cli::{AppState, router};
use kira_client::Client;
use kira_config::CliConfig;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = CliConfig::from_env().context("loading CLI configuration")?;
    let client = Client::new(config.remote_url.clone());

    let listener = TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("binding {}", config.bind))?;
    let address = listener
        .local_addr()
        .context("reading the local listener address")?;
    println!("Listening on http://{address}");
    println!("Proxying to {}", config.remote_url);

    axum::serve(listener, router(AppState { client }))
        .with_graceful_shutdown(async {
            if let Err(error) = kira_core::shutdown::wait_for_signal().await {
                eprintln!("failed to wait for shutdown signal: {error}");
            }
        })
        .await
        .context("serving the local dashboard")?;

    Ok(())
}
