//! Kira service binary: Discord bot plus HTTP and WebSocket API.

mod app;

use std::env;
use std::process::ExitCode;

use anyhow::Context;
use kira_config::DiscordConfig;
#[cfg(feature = "server")]
use kira_config::ServerConfig;

#[tokio::main]
async fn main() -> ExitCode {
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
            eprintln!("Unknown argument: {other}");
            eprintln!("Usage: kira [--version] [--help]");
            ExitCode::from(2)
        }
        None => match run().await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error:#}");
                ExitCode::FAILURE
            }
        },
    }
}

async fn run() -> anyhow::Result<()> {
    let discord = DiscordConfig::from_env().context("loading Discord configuration")?;

    #[cfg(feature = "server")]
    {
        let server = ServerConfig::from_env().context("loading server configuration")?;
        app::run_all(discord, server).await
    }

    #[cfg(not(feature = "server"))]
    {
        app::run_bot(discord).await
    }
}
