//! Shared runtime utilities for the Kira service.

/// Outgoing Discord message requests shared by the bot and the API server.
pub mod messaging;

/// Wait for termination signals from the operating system.
pub mod shutdown;
