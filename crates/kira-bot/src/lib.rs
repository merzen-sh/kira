//! Discord bot runtime for the Kira service.

mod bot;
mod channels;
mod commands;
mod errors;
mod guilds;
pub mod handler;
mod members;
mod state;
mod sync;

pub use bot::run;
pub use errors::BotError;
pub use handler::{ChannelsKey, ConfigKey, Handler, is_ping};
