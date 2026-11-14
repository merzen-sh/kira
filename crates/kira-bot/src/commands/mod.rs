//! Slash commands defined and handled by the bot.

mod ping;
mod report;

use serenity::builder::CreateCommand;
use serenity::model::application::{Command, CommandInteraction, ModalInteraction};
use serenity::model::id::GuildId;
use serenity::prelude::Context;

/// Register the bot's slash commands in `guild_id`, or globally when unset.
pub(crate) async fn register(
    ctx: &Context,
    guild_id: Option<u64>,
) -> Result<(), Box<serenity::Error>> {
    let commands: Vec<CreateCommand> = vec![ping::command(), report::command()];

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
        report::NAME => report::open(ctx, command).await,
        _ => Ok(()),
    }
}

/// Handle a submitted modal, ignoring modals this bot does not own.
pub(crate) async fn handle_modal(
    ctx: &Context,
    modal: &ModalInteraction,
) -> Result<(), Box<serenity::Error>> {
    match modal.data.custom_id.as_str() {
        report::MODAL_ID => report::submit(ctx, modal).await,
        _ => Ok(()),
    }
}
