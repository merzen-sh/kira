//! The `/ping` slash command.

use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::prelude::Context;

use crate::errors::BotError;

/// Name of the `/ping` slash command.
pub(super) const NAME: &str = "ping";

pub(super) fn command() -> CreateCommand {
    CreateCommand::new(NAME).description("Check that the bot is alive.")
}

pub(super) async fn respond(ctx: &Context, command: &CommandInteraction) -> Result<(), BotError> {
    let response =
        CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("pong"));

    command.create_response(&ctx.http, response).await?;

    Ok(())
}

/// Return whether a message is a `!ping` command from a human.
#[must_use]
pub fn is_ping(content: &str, author_is_bot: bool) -> bool {
    !author_is_bot && content == "!ping"
}

#[cfg(test)]
mod tests {
    use super::*;

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
