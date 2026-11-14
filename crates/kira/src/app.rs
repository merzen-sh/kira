use anyhow::{Context, Result};
use kira_config::DatabaseConfig;
use kira_config::DiscordConfig;
#[cfg(feature = "server")]
use kira_config::ServerConfig;
use tokio::sync::{mpsc, watch};

async fn wait_flag(mut rx: watch::Receiver<bool>) {
    loop {
        if *rx.borrow_and_update() {
            return;
        }

        if rx.changed().await.is_err() {
            return;
        }
    }
}

#[cfg(feature = "server")]
pub async fn run_all(
    discord: DiscordConfig,
    server: ServerConfig,
    database: DatabaseConfig,
) -> Result<()> {
    let reports = kira_store::Store::connect(&database.url)
        .await
        .context("opening report database")?;
    let (tx, rx) = watch::channel(false);
    let (guild_tx, guild_rx) = watch::channel(Vec::new());
    let (channel_tx, channel_rx) = watch::channel(Vec::new());
    let (message_tx, message_rx) = mpsc::channel(16);

    let bot = kira_bot::run(
        discord,
        Some(server.guild_id),
        guild_tx,
        channel_tx,
        reports.clone(),
        message_rx,
        wait_flag(rx.clone()),
    );
    let http = kira_server::serve(
        &server,
        guild_rx,
        channel_rx,
        reports,
        message_tx,
        wait_flag(rx.clone()),
    );
    tokio::pin!(bot);
    tokio::pin!(http);

    let mut bot_result = None;
    let mut http_result = None;
    let mut signal_result = None;

    tokio::select! {
        result = &mut bot => {
            bot_result = Some(result);
        }
        result = &mut http => {
            http_result = Some(result);
        }
        signal = kira_core::shutdown::wait_for_signal() => {
            signal_result = Some(signal);
        }
    }

    tx.send_replace(true);

    let bot_result = match bot_result {
        Some(result) => result,
        None => bot.await,
    };
    let http_result = match http_result {
        Some(result) => result,
        None => http.await,
    };

    if let Some(signal) = signal_result {
        signal.context("waiting for shutdown signal")?;
    }

    bot_result.context("Discord bot failed")?;
    http_result.context("HTTP server failed")?;

    Ok(())
}

#[cfg(not(feature = "server"))]
pub async fn run_bot(discord: DiscordConfig, database: DatabaseConfig) -> Result<()> {
    let reports = kira_store::Store::connect(&database.url)
        .await
        .context("opening report database")?;
    let (tx, rx) = watch::channel(false);
    let (guild_tx, _guild_rx) = watch::channel(Vec::new());
    let (channel_tx, _channel_rx) = watch::channel(Vec::new());
    let (_message_tx, message_rx) = mpsc::channel(1);

    let bot = kira_bot::run(
        discord,
        None,
        guild_tx,
        channel_tx,
        reports,
        message_rx,
        wait_flag(rx.clone()),
    );
    tokio::pin!(bot);

    let mut bot_result = None;
    let mut signal_result = None;

    tokio::select! {
        result = &mut bot => {
            bot_result = Some(result);
        }
        signal = kira_core::shutdown::wait_for_signal() => {
            signal_result = Some(signal);
        }
    }

    tx.send_replace(true);

    let bot_result = match bot_result {
        Some(result) => result,
        None => bot.await,
    };

    if let Some(signal) = signal_result {
        signal.context("waiting for shutdown signal")?;
    }

    bot_result.context("Discord bot failed")
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use kira_server::ServerError;

    #[test]
    fn error_context_is_not_duplicated() {
        let error = anyhow::Error::new(ServerError::Io(std::io::Error::other(
            "address already in use",
        )))
        .context("HTTP server failed");

        let rendered = format!("{error:#}");
        assert_eq!(rendered.matches("address already in use").count(), 1);
    }

    #[test]
    fn bind_error_context_names_the_address_and_renders_once() {
        let error = anyhow::Error::new(ServerError::Bind {
            addr: std::net::SocketAddr::from(([127, 0, 0, 1], 8080)),
            source: std::io::Error::other("address already in use"),
        })
        .context("HTTP server failed");

        let rendered = format!("{error:#}");
        assert!(rendered.contains("127.0.0.1:8080"));
        assert_eq!(rendered.matches("address already in use").count(), 1);
    }
}
