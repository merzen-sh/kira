//! HTTP router assembly.

use axum::{Router, middleware, routing};

use crate::handlers::ws::ws_upgrade;
use crate::handlers::{
    delete_report, get_guild, get_guild_channels, guild_user_stats, health, list_guild_users,
    list_reports, login, logout, report_stats, send_guild_message, session, system_monitor,
    update_report,
};
use crate::middleware::{cors_layer, record_peer, require_origin, require_session};
#[cfg(debug_assertions)]
use crate::openapi::{openapi_json, scalar_ui};
use crate::state::AppState;

/// Build the HTTP router for the Kira API.
pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/auth/session", routing::get(session))
        .route("/guild", routing::get(get_guild))
        .route("/guild/channels", routing::get(get_guild_channels))
        .route("/guild/users", routing::get(list_guild_users))
        .route("/guild/users/stats", routing::get(guild_user_stats))
        .route("/monitor", routing::get(system_monitor))
        .route(
            "/guild/channels/{channel_id}/messages",
            routing::post(send_guild_message),
        )
        .route("/reports", routing::get(list_reports))
        .route("/reports/stats", routing::get(report_stats))
        .route(
            "/reports/{id}",
            routing::patch(update_report).delete(delete_report),
        )
        .route("/ws", routing::get(ws_upgrade))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_session,
        ));

    let router = Router::new()
        .route("/health", routing::get(health))
        .route("/auth/login", routing::post(login))
        .route("/auth/logout", routing::post(logout))
        .merge(protected)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_origin,
        ))
        .layer(middleware::from_fn(record_peer))
        .layer(cors_layer(&state))
        .with_state(state);

    #[cfg(debug_assertions)]
    let router = router
        .route("/docs", routing::get(scalar_ui))
        .route("/openapi.json", routing::get(openapi_json));

    router
}
