use kira_cli::{AppState, router};
use kira_client::Client;
use kira_config::CliConfig;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> kira_core::Result<()> {
    let config = CliConfig::from_env()?;
    let client = Client::new(config.remote_url.clone());

    let listener = TcpListener::bind(config.bind).await?;
    println!("Listening on http://{}", listener.local_addr()?);
    println!("Proxying to {}", config.remote_url);

    axum::serve(listener, router(AppState { client }))
        .with_graceful_shutdown(async {
            let _ = kira_core::shutdown::wait_for_signal().await;
        })
        .await?;

    Ok(())
}
