//! The `/ping` slash command.

use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::prelude::Context;

/// Name of the `/ping` slash command.
pub(super) const NAME: &str = "ping";

pub(super) fn command() -> CreateCommand {
    CreateCommand::new(NAME).description("Check that the bot is alive.")
}

pub(super) async fn respond(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), Box<serenity::Error>> {
    let response =
        CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("pong"));

    command.create_response(&ctx.http, response).await?;

    Ok(())
}
