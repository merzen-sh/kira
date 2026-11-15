//! Environment-backed configuration for Kira binaries.
//!
//! Provides configuration types for Discord, the HTTP server, and the local
//! dashboard. Configuration can be loaded from process environment variables
//! or parsed from an iterator for testing.
//!
//! Environment variables use the following prefixes:
//! - `DISCORD_` for Discord configuration.
//! - `KIRA_` for server and CLI configuration.

mod cli;
mod database;
mod discord;
mod env;
mod errors;
mod server;

pub use cli::CliConfig;
pub use database::DatabaseConfig;
pub use discord::DiscordConfig;
pub use env::load_env;
pub use errors::ConfigError;
pub use server::{SameSite, ServerConfig};
