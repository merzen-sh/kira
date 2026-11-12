//! Discord gateway runtime.

use std::future::Future;
use std::sync::Arc;

use kira_config::DiscordConfig;
use kira_core::messaging::OutgoingMessage;
use kira_protocol::{ChannelDetails, GuildDetails};
use serenity::Client;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use serenity::prelude::GatewayIntents;
use thiserror::Error;
use tokio::sync::{mpsc, watch};

use crate::handler::{ChannelsKey, ConfigKey, GuildsKey, Handler};

/// Errors produced while running the Discord bot.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BotError {
    /// Serenity failed to build or run the Discord client.
    #[error("Discord client error")]
    Serenity(#[source] Box<serenity::Error>),
}

impl From<serenity::Error> for BotError {
    fn from(error: serenity::Error) -> Self {
        Self::Serenity(Box::new(error))
    }
}

/// Run the Discord bot until the gateway stops or `shutdown` resolves.
pub async fn run(
    config: DiscordConfig,
    guilds: watch::Sender<Vec<GuildDetails>>,
    channels: watch::Sender<Vec<ChannelDetails>>,
    messages: mpsc::Receiver<OutgoingMessage>,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), BotError> {
    let config = Arc::new(config);

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&config.token, intents)
        .event_handler(Handler)
        .await?;

    // Register shared state.
    {
        let mut data = client.data.write().await;
        data.insert::<ConfigKey>(Arc::clone(&config));
        data.insert::<GuildsKey>(guilds);
        data.insert::<ChannelsKey>(channels);
    }

    spawn_message_sender(client.http.clone(), messages);

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

fn spawn_message_sender(http: Arc<Http>, mut messages: mpsc::Receiver<OutgoingMessage>) {
    tokio::spawn(async move {
        while let Some(message) = messages.recv().await {
            let result = send_message(&http, &message).await;
            let _ = message.reply.send(result);
        }
    });
}

async fn send_message(http: &Http, message: &OutgoingMessage) -> Result<String, String> {
    let channel_id = message
        .channel_id
        .parse::<u64>()
        .map_err(|error| format!("invalid channel ID: {error}"))?;
    let body = &message.payload;
    let sent = http
        .send_message(ChannelId::new(channel_id), Vec::new(), &body)
        .await
        .map_err(|error| error.to_string())?;

    Ok(sent.id.to_string())
}

async fn wait_for_gateway<F>(shards_running: bool, start: F) -> Result<(), BotError>
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
    async fn no_shards_does_not_await_gateway() -> Result<(), Box<dyn std::error::Error>> {
        let never = std::future::pending::<serenity::Result<()>>();

        let result =
            tokio::time::timeout(Duration::from_millis(100), wait_for_gateway(false, never))
                .await?;

        result?;
        Ok(())
    }

    #[tokio::test]
    async fn started_shards_await_gateway() -> Result<(), Box<dyn std::error::Error>> {
        let ready = async { Ok::<(), serenity::Error>(()) };

        let result =
            tokio::time::timeout(Duration::from_millis(100), wait_for_gateway(true, ready)).await?;

        result?;
        Ok(())
    }
}
