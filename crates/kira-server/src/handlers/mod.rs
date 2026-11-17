//! HTTP route handlers grouped by API domain.

pub(crate) mod audit;
pub(crate) mod auth;
pub(crate) mod guild;
pub(crate) mod health;
pub(crate) mod monitor;
pub(crate) mod presence;
pub(crate) mod reports;
pub(crate) mod users;
pub(crate) mod ws;

pub use audit::{audit_facets, list_audit_events};
pub use auth::{login, logout, session};
pub use guild::{guild, guild_channels, send_guild_message};
pub use health::health;
pub use monitor::system_monitor;
pub use presence::{get_presence, update_presence};
pub use reports::{delete_report, list_reports, report_stats, update_report};
pub use users::{guild_user_stats, list_guild_users};

/// Default number of records returned by paginated endpoints.
const DEFAULT_PAGE_LIMIT: u32 = 20;

/// Largest page size accepted by paginated endpoints.
const MAX_PAGE_LIMIT: u32 = 100;

pub(crate) fn pagination(page: Option<u32>, page_limit: Option<u32>) -> (u32, u32) {
    let page = page.unwrap_or(1).max(1);
    let page_limit = page_limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .clamp(1, MAX_PAGE_LIMIT);
    (page, page_limit)
}
