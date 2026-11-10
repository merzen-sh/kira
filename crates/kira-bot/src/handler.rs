//! Discord event handling for the `!ping` command.

use std::sync::Arc;

use kira_config::DiscordConfig;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::{Context, EventHandler, TypeMapKey};

/// Type-map key holding the shared Discord configuration.
#[non_exhaustive]
pub struct ConfigKey;

impl TypeMapKey for ConfigKey {
    type Value = Arc<DiscordConfig>;
}

/// Serenity event handler for the bot.
#[non_exhaustive]
pub struct Handler;

/// Return whether a message is a `!ping` command from a human.
#[must_use]
pub fn is_ping(content: &str, author_is_bot: bool) -> bool {
    !author_is_bot && content == "!ping"
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !is_ping(
            msg.content.as_str(),
            /* author_is_bot= */ msg.author.bot,
        ) {
            return;
        }

        // Access shared configuration.
        let config = {
            let data = ctx.data.read().await;
            data.get::<ConfigKey>().cloned()
        };

        if config.is_some() {
            println!("Responding to ping!");
        }

        if let Err(error) = msg.channel_id.say(&ctx.http, "Pong!").await {
            eprintln!("Failed to send message: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_ping;

    #[test]
    fn ping_from_human_matches() {
        assert!(is_ping("!ping", false));
    }

    #[test]
    fn ping_from_bot_is_ignored() {
        assert!(!is_ping("!ping", true));
    }

    #[test]
    fn other_text_is_ignored() {
        assert!(!is_ping("hello", false));
    }
}
