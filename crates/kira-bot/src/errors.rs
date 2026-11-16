//! Errors produced while running the Discord bot.

use kira_store::StoreError;
use thiserror::Error;

/// Errors produced while running the Discord bot.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BotError {
    /// Serenity failed to build or run the Discord client.
    #[error("discord client error")]
    Serenity(#[source] Box<serenity::Error>),

    /// Guild membership storage failed.
    #[error("storage error")]
    Store(#[source] StoreError),
}

impl From<serenity::Error> for BotError {
    fn from(error: serenity::Error) -> Self {
        Self::Serenity(Box::new(error))
    }
}

impl From<StoreError> for BotError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}
