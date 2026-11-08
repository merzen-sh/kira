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

            // Gracefully stop all Discord gateway shards.
            shard_manager.shutdown_all().await;

            // Wait for the gateway to finish shutting down.
            start.await?;

            println!("Bot shutdown completed.");
        }
    }

    Ok(())
}
