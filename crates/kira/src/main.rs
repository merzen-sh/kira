//! Kira service binary: Discord bot plus HTTP and WebSocket API.

mod app;

use std::env;
use std::process::ExitCode;

use anyhow::Context;
use kira_config::DatabaseConfig;
use kira_config::DiscordConfig;
#[cfg(feature = "server")]
use kira_config::ServerConfig;

#[tokio::main]
async fn main() -> ExitCode {
    kira_core::logging::init();

    let mut args = env::args().skip(1);

    match args.next().as_deref() {
        Some("--version" | "-V") => {
            println!("kira {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help" | "-h") => {
            println!("Usage: kira [--version] [--help]");
            ExitCode::SUCCESS
        }
        Some(other) => {
            tracing::error!("Unknown argument: {other}");
            tracing::error!("Usage: kira [--version] [--help]");
            ExitCode::from(2)
        }
        None => match run().await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                tracing::error!("{error:#}");
                ExitCode::FAILURE
            }
        },
    }
}

async fn run() -> anyhow::Result<()> {
    let discord = DiscordConfig::from_env().context("loading Discord configuration")?;
    let database = DatabaseConfig::from_env().context("loading database configuration")?;

    #[cfg(feature = "server")]
    {
        let server = ServerConfig::from_env().context("loading server configuration")?;
        app::run_all(discord, server, database).await
    }

    #[cfg(not(feature = "server"))]
    {
        app::run_bot(discord, database).await
    }
}
