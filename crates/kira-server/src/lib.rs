//! HTTP and WebSocket server for the Kira API.

mod auth;
mod errors;
mod handlers;
mod middleware;
mod monitor;
mod openapi;
mod router;
mod serve;
mod state;

pub use auth::{Auth, AuthConfig, COOKIE_NAME, SESSION_TTL};
pub use errors::{LoginError, ServerError};
pub use handlers::audit::AuditListQuery;
pub use handlers::monitor::MonitorQuery;
pub use handlers::reports::ReportListQuery;
pub use handlers::users::UserListQuery;
pub use handlers::{
    audit_facets, delete_report, guild, guild_channels, guild_user_stats, health,
    list_audit_events, list_guild_users, list_reports, login, logout, report_stats,
    send_guild_message, session, system_monitor, update_report,
};
pub use middleware::PeerAddress;
pub use openapi::ApiDoc;
pub use router::router;
pub use serve::{serve, serve_with_listener};
pub use state::AppState;
