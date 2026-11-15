//! Shared state for the HTTP server.

use kira_core::messaging::MessageSender;
use kira_protocol::{ChannelDetails, GuildDetails};
use kira_store::Database;
use tokio::sync::watch;

use crate::auth::Auth;

/// Shared state for the HTTP server.
#[derive(Clone)]
pub struct AppState {
    guild_id: String,
    guilds: watch::Receiver<Vec<GuildDetails>>,
    channels: watch::Receiver<Vec<ChannelDetails>>,
    database: Database,
    messages: MessageSender,
    auth: Auth,
}

impl AppState {
    /// Build state from the configured guild ID, the bot's cached state,
    /// database storage, and the authentication settings.
    #[must_use]
    pub fn new(
        guild_id: u64,
        guilds: watch::Receiver<Vec<GuildDetails>>,
        channels: watch::Receiver<Vec<ChannelDetails>>,
        database: Database,
        messages: MessageSender,
        auth: Auth,
    ) -> Self {
        Self {
            guild_id: guild_id.to_string(),
            guilds,
            channels,
            database,
            messages,
            auth,
        }
    }

    pub(crate) fn database(&self) -> &Database {
        &self.database
    }

    pub(crate) fn messages(&self) -> &MessageSender {
        &self.messages
    }

    pub(crate) fn auth(&self) -> &Auth {
        &self.auth
    }

    pub(crate) fn guild(&self) -> Option<GuildDetails> {
        self.guilds
            .borrow()
            .iter()
            .find(|guild| guild.id == self.guild_id)
            .cloned()
    }

    pub(crate) fn guild_channels(&self) -> Option<Vec<ChannelDetails>> {
        self.guild()?;

        Some(
            self.channels
                .borrow()
                .iter()
                .filter(|channel| channel.guild_id == self.guild_id)
                .cloned()
                .collect(),
        )
    }
}
