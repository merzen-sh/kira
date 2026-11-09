use anyhow::Context;
use kira_cli::{AppState, router};
use kira_client::Client;
use kira_config::CliConfig;
use tokio::net::TcpListener;

const VITE_DEV_SERVER_URL: &str = "http://127.0.0.1:5173";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = CliConfig::from_env().context("loading CLI configuration")?;
    let client = Client::new(config.remote_url.clone());
    let dev_server_url = cfg!(debug_assertions).then(|| VITE_DEV_SERVER_URL.to_owned());

    let listener = TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("binding {}", config.bind))?;
    let address = listener
        .local_addr()
        .context("reading the local listener address")?;
    println!("Listening on http://{address}");
    println!("Proxying to {}", config.remote_url);

    axum::serve(
        listener,
        router(AppState {
            client,
            dev_server_url,
        }),
    )
    .with_graceful_shutdown(async {
        if let Err(error) = kira_core::shutdown::wait_for_signal().await {
            eprintln!("failed to wait for shutdown signal: {error}");
        }
    })
    .await
    .context("serving the local dashboard")?;

    Ok(())
}
