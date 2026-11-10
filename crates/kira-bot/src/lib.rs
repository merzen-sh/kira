//! Discord bot runtime for the Kira service.

mod bot;
pub mod handler;

pub use bot::{BotError, run};
pub use handler::{ConfigKey, Handler, is_ping};
