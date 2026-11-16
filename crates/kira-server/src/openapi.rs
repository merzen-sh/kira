//! OpenAPI document and debug API reference routes.

#[cfg(debug_assertions)]
use axum::Json;
#[cfg(debug_assertions)]
use axum::response::Html;
#[cfg(debug_assertions)]
use utoipa::OpenApi;

use crate::auth::COOKIE_NAME;

/// OpenAPI document assembled from the server's route annotations.
#[derive(utoipa::OpenApi)]
#[non_exhaustive]
#[openapi(
    info(title = "Kira API", version = env!("CARGO_PKG_VERSION"), description = "Kira service API"),
    tags(
        (name = "auth", description = "Dashboard session authentication"),
        (name = "health", description = "Service health"),
        (name = "audit", description = "Generic audit event log"),
        (name = "monitor", description = "Host CPU, memory, and disk usage"),
        (name = "guild", description = "Discord server and channels"),
        (name = "reports", description = "User-submitted bug reports")
    ),
    paths(
        crate::handlers::health::health,
        crate::handlers::monitor::system_monitor,
        crate::handlers::audit::list_audit_events,
        crate::handlers::audit::audit_facets,
        crate::handlers::auth::login,
        crate::handlers::auth::logout,
        crate::handlers::auth::session,
        crate::handlers::guild::get_guild,
        crate::handlers::guild::get_guild_channels,
        crate::handlers::guild::send_guild_message,
        crate::handlers::users::list_guild_users,
        crate::handlers::users::guild_user_stats,
        crate::handlers::reports::list_reports,
        crate::handlers::reports::report_stats,
        crate::handlers::reports::update_report,
        crate::handlers::reports::delete_report
    ),
    components(schemas(
            kira_protocol::HealthResponse,
            kira_protocol::MonitorReport,
            kira_protocol::MonitorSample,
            kira_protocol::AuditEntity,
            kira_protocol::AuditEvent,
            kira_protocol::AuditEventList,
            kira_protocol::AuditFacets,
            kira_protocol::GuildDetails,
            kira_protocol::ChannelDetails,
            kira_protocol::SentMessage,
            kira_protocol::GuildUser,
            kira_protocol::GuildUserList,
            kira_protocol::GuildUserStats,
            kira_protocol::LoginRequest,
            kira_protocol::Report,
            kira_protocol::ReportList,
            kira_protocol::ReportStats,
            kira_protocol::UpdateReport,
            kira_protocol::ErrorResponse
        )),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};

        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "cookie_auth",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(COOKIE_NAME))),
            );
        }
    }
}

#[cfg(debug_assertions)]
const SCALAR_UI: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Kira API Reference</title>
  </head>
  <body>
    <script id="api-reference" data-url="/openapi.json"></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
  </body>
</html>
"#;

#[cfg(debug_assertions)]
pub(crate) async fn scalar_ui() -> Html<&'static str> {
    Html(SCALAR_UI)
}

#[cfg(debug_assertions)]
pub(crate) async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
