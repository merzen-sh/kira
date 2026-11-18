use std::future::Future;

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

async fn record_started(database: &kira_store::Database) {
    let actor = kira_protocol::AuditEntity::new("system", None, Some("Kira".to_owned()));
    let mut metadata = serde_json::Map::new();
    metadata.insert("version".to_owned(), env!("CARGO_PKG_VERSION").into());
    let event = kira_store::audit::NewAuditEvent::new(
        "system.started",
        kira_protocol::SOURCE_SYSTEM,
        &actor,
        None,
        &metadata,
    );

    if let Err(error) = database.audit().record(&event).await {
        tracing::error!(
            action = event.action,
            source = event.source,
            actor_kind = event.actor.kind,
            error = ?error,
            "failed to record startup audit event"
        );
    }
}

/// Open storage, record the startup event, and load the configured presence.
async fn initialize(
    config: &DatabaseConfig,
) -> Result<(kira_store::Database, kira_protocol::BotPresence)> {
    let database = kira_store::Database::connect(&config.url)
        .await
        .context("opening database")?;
    record_started(&database).await;
    let presence = database
        .presence()
        .get()
        .await
        .context("loading bot presence")?;

    Ok((database, presence))
}

/// Run `services` until it finishes or a shutdown signal arrives, then request
/// shutdown and let `services` drain.
async fn run_until_shutdown<F>(shutdown: &watch::Sender<bool>, services: F) -> Result<()>
where
    F: Future<Output = Result<()>>,
{
    tokio::pin!(services);

    let mut services_result = None;
    let mut signal_result = None;

    tokio::select! {
        result = &mut services => {
            services_result = Some(result);
        }
        signal = kira_core::shutdown::wait_for_signal() => {
            signal_result = Some(signal);
        }
    }

    shutdown.send_replace(true);

    let services_result = match services_result {
        Some(result) => result,
        None => services.await,
    };

    if let Some(signal) = signal_result {
        signal.context("waiting for shutdown signal")?;
    }

    services_result
}

#[cfg(feature = "server")]
pub async fn run_all(
    discord: DiscordConfig,
    server: ServerConfig,
    database: DatabaseConfig,
) -> Result<()> {
    let (database, presence) = initialize(&database).await?;
    let (tx, rx) = watch::channel(false);
    let (guild_tx, guild_rx) = watch::channel(Vec::new());
    let (channel_tx, channel_rx) = watch::channel(Vec::new());
    let (presence_tx, presence_rx) = watch::channel(presence);
    let (message_tx, message_rx) = mpsc::channel(16);

    let bot = kira_bot::run(
        discord,
        Some(server.guild_id),
        guild_tx,
        channel_tx,
        presence_rx,
        database.clone(),
        message_rx,
        wait_flag(rx.clone()),
    );
    let http = kira_server::serve(
        &server,
        guild_rx,
        channel_rx,
        database,
        message_tx,
        presence_tx,
        wait_flag(rx),
    );
    run_until_shutdown(&tx, async {
        tokio::pin!(bot);
        tokio::pin!(http);

        tokio::select! {
            bot_result = &mut bot => {
                tx.send_replace(true);
                let http_result = http.await;
                bot_result.context("Discord bot failed")?;
                http_result.context("HTTP server failed")
            }
            http_result = &mut http => {
                tx.send_replace(true);
                let bot_result = bot.await;
                bot_result.context("Discord bot failed")?;
                http_result.context("HTTP server failed")
            }
        }
    })
    .await
}

#[cfg(not(feature = "server"))]
pub async fn run_bot(discord: DiscordConfig, database: DatabaseConfig) -> Result<()> {
    let (database, presence) = initialize(&database).await?;
    let (tx, rx) = watch::channel(false);
    let (guild_tx, _guild_rx) = watch::channel(Vec::new());
    let (channel_tx, _channel_rx) = watch::channel(Vec::new());
    let (_presence_tx, presence_rx) = watch::channel(presence);
    let (_message_tx, message_rx) = mpsc::channel(1);

    let bot = kira_bot::run(
        discord,
        None,
        guild_tx,
        channel_tx,
        presence_rx,
        database,
        message_rx,
        wait_flag(rx),
    );

    run_until_shutdown(&tx, async move { bot.await.context("Discord bot failed") }).await
}

#[cfg(test)]
mod system_audit_tests {
    use super::{initialize, record_started};

    #[tokio::test]
    async fn initialize_loads_presence() -> Result<(), Box<dyn std::error::Error>> {
        let config =
            kira_config::DatabaseConfig::try_from_iter([("KIRA_DATABASE_URL", "sqlite::memory:")])?;

        let (database, presence) = initialize(&config).await?;

        assert_eq!(presence, kira_protocol::BotPresence::default());
        let events = database
            .audit()
            .list_events(1, 20, None, None, None, None)
            .await?;
        assert_eq!(events.total, 1);
        Ok(())
    }

    #[tokio::test]
    async fn startup_records_a_system_event() -> Result<(), Box<dyn std::error::Error>> {
        let database = kira_store::Database::connect("sqlite::memory:").await?;

        record_started(&database).await;

        let events = database
            .audit()
            .list_events(1, 20, None, None, None, None)
            .await?;
        assert_eq!(events.total, 1);
        let event = &events.events[0];
        assert_eq!(event.action, "system.started");
        assert_eq!(event.source, "system");
        assert_eq!(event.actor.kind, "system");
        assert_eq!(event.actor.name.as_deref(), Some("Kira"));
        assert_eq!(event.metadata["version"], env!("CARGO_PKG_VERSION"));
        Ok(())
    }
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
