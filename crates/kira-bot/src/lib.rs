//! Discord bot runtime for the Kira service.

mod bot;
mod commands;
pub mod handler;

pub use bot::{BotError, run};
pub use handler::{ChannelsKey, ConfigKey, Handler, is_ping};
