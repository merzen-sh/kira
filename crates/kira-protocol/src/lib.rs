//! Request and response types shared by the Kira server and its clients.

mod audit;
mod auth;
mod error;
mod guild;
mod health;
mod monitor;
mod presence;
mod reports;

pub use audit::{
    AuditEntity, AuditEvent, AuditEventList, AuditFacets, SOURCE_BOT, SOURCE_DASHBOARD,
    SOURCE_SYSTEM,
};
pub use auth::LoginRequest;
pub use error::ErrorResponse;
pub use guild::{
    ChannelDetails, GuildDetails, GuildUser, GuildUserList, GuildUserStats, SentMessage,
};
pub use health::HealthResponse;
pub use monitor::{MonitorReport, MonitorSample};
pub use presence::{ActivityKind, BotActivity, BotPresence, BotStatus};
pub use reports::{Report, ReportList, ReportStats, UpdateReport};
