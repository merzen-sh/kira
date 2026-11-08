pub mod handler;

use std::{future::Future, sync::Arc};

use kira_config::DiscordConfig;
use serenity::{Client, prelude::GatewayIntents};

pub use handler::{ConfigKey, Handler, is_ping};

pub async fn run(
    config: DiscordConfig,
    shutdown: impl Future<Output = ()> + Send,
) -> kira_core::Result<()> {
    let config = Arc::new(config);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&config.token, intents)
        .event_handler(Handler)
        .await?;

    // Register shared state.
    {
        let mut data = client.data.write().await;
        data.insert::<ConfigKey>(Arc::clone(&config));
    }

    // Keep a reference to the shard manager.
    let shard_manager = Arc::clone(&client.shard_manager);

    // Pin the gateway future so it can be awaited after receiving a signal.
    let start = client.start();
    tokio::pin!(start);

    tokio::select! {
        result = &mut start => {
            result?;
        }

        () = shutdown => {
            println!("Shutdown signal received.");

            // `shutdown_all` is a no-op when no shard runner exists, and the
            // gateway future would then keep booting forever. Only wait for it
            // when shards were actually instantiated; otherwise dropping
            // `start` cancels the pending gateway boot.
            let shards_running = !shard_manager.shards_instantiated().await.is_empty();

            // Gracefully stop all Discord gateway shards.
            shard_manager.shutdown_all().await;

            // Wait for the gateway to finish shutting down.
            wait_for_gateway(shards_running, start).await?;

            println!("Bot shutdown completed.");
        }
    }

    Ok(())
}

async fn wait_for_gateway<F>(shards_running: bool, start: F) -> kira_core::Result<()>
where
    F: Future<Output = serenity::Result<()>>,
{
    if shards_running {
        start.await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::wait_for_gateway;

    #[tokio::test]
    async fn no_shards_does_not_await_gateway() {
        let never = std::future::pending::<serenity::Result<()>>();

        let result =
            tokio::time::timeout(Duration::from_millis(100), wait_for_gateway(false, never))
                .await
                .expect("gateway must not be awaited when no shards are running");

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn started_shards_await_gateway() {
        let ready = async { Ok(()) };

        let result =
            tokio::time::timeout(Duration::from_millis(100), wait_for_gateway(true, ready))
                .await
                .expect("gateway future should be awaited when shards are running");

        assert!(result.is_ok());
    }
}
