use std::sync::Arc;

use serenity::{async_trait, model::channel::Message, prelude::*};

use crate::config::Config;

pub struct ConfigKey;

impl TypeMapKey for ConfigKey {
    type Value = Arc<Config>;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot || msg.content != "!ping" {
            return;
        }

        // Access shared configuration.
        let config = {
            let data = ctx.data.read().await;
            data.get::<ConfigKey>().cloned()
        };

        if let Some(_config) = config {
            println!("Responding to ping!");
        }

        if let Err(error) = msg.channel_id.say(&ctx.http, "Pong!").await {
            eprintln!("Failed to send message: {error}");
        }
    }
}
