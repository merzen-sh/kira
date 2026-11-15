//! Errors produced while running the Discord bot.

use thiserror::Error;

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
