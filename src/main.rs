mod config;
mod handler;
mod shutdown;

use std::{error::Error, sync::Arc};

use serenity::prelude::*;
use tokio::runtime::Builder;

use config::Config;
use handler::{ConfigKey, Handler};

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = Builder::new_multi_thread().enable_all().build()?;

    runtime.block_on(async {
        let config = Arc::new(Config::from_env()?);

        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let mut client = Client::builder(&config.discord_token, intents)
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

            signal = shutdown::wait_for_signal() => {
                signal?;

                println!("Shutdown signal received.");

                // Gracefully stop all Discord gateway shards.
                shard_manager.shutdown_all().await;

                // Wait for the gateway to finish shutting down.
                start.await?;

                println!("Bot shutdown completed.");
            }
        }

        Ok::<(), Box<dyn Error>>(())
    })
}
