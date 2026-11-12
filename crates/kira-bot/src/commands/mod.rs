//! Slash commands defined and handled by the bot.

mod ping;

use serenity::builder::CreateCommand;
use serenity::model::application::{Command, CommandInteraction};
use serenity::model::id::GuildId;
use serenity::prelude::Context;

/// Register the bot's slash commands in `guild_id`, or globally when unset.
pub(crate) async fn register(
    ctx: &Context,
    guild_id: Option<u64>,
) -> Result<(), Box<serenity::Error>> {
    let commands: Vec<CreateCommand> = vec![ping::command()];

    match guild_id {
        Some(guild_id) => {
            ctx.http
                .create_guild_commands(GuildId::new(guild_id), &commands)
                .await?;
        }
        None => {
            Command::set_global_commands(&ctx.http, commands).await?;
        }
    }

    Ok(())
}

/// Answer a command interaction, ignoring commands this bot does not own.
pub(crate) async fn handle(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), Box<serenity::Error>> {
    match command.data.name.as_str() {
        ping::NAME => ping::respond(ctx, command).await,
        _ => Ok(()),
    }
}
