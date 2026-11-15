//! Server binding and lifecycle.

use std::future::Future;
use std::net::SocketAddr;

use kira_config::ServerConfig;
use kira_core::messaging::MessageSender;
use kira_protocol::{ChannelDetails, GuildDetails};
use kira_store::Database;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::auth::{Auth, AuthConfig};
use crate::errors::ServerError;
use crate::state::AppState;
use crate::{monitor, router};

/// Bind to the configured address and serve until `shutdown` resolves.
pub async fn serve(
    config: &ServerConfig,
    guilds: watch::Receiver<Vec<GuildDetails>>,
    channels: watch::Receiver<Vec<ChannelDetails>>,
    database: Database,
    messages: MessageSender,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServerError> {
    ensure_secure_bind(
        config.bind,
        config.https,
        config.cookie_secure,
        config.allowed_origins.is_some(),
    )?;

    let auth = Auth::new(AuthConfig::from_server_config(config)?)?;
    if let Some(pin) = auth.generated_pin() {
        tracing::warn!("KIRA_PASSWORD is not set; generated dashboard PIN: {pin}");
    }

    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|source| ServerError::Bind {
            addr: config.bind,
            source,
        })?;
    tracing::info!("Listening on http://{}", listener.local_addr()?);

    serve_with_listener(
        listener,
        AppState::new(config.guild_id, guilds, channels, database, messages, auth),
        shutdown,
    )
    .await
}

fn ensure_secure_bind(
    bind: SocketAddr,
    https: bool,
    cookie_secure: bool,
    has_allowed_origins: bool,
) -> Result<(), ServerError> {
    if bind.ip().is_loopback() || (https && cookie_secure && has_allowed_origins) {
        return Ok(());
    }

    Err(ServerError::InsecureBind { addr: bind })
}

/// Serve on an already-bound listener until `shutdown` resolves.
pub async fn serve_with_listener(
    listener: TcpListener,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServerError> {
    let sampler = tokio::spawn(monitor::run(state.database().clone()));
    let result = axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await;
    sampler.abort();
    result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_bind_needs_no_origin_configuration() {
        assert!(
            ensure_secure_bind(
                SocketAddr::from(([127, 0, 0, 1], 8080)),
                false,
                false,
                false
            )
            .is_ok()
        );
    }

    #[test]
    fn public_bind_requires_https_secure_cookies_and_an_allowlist() {
        let public = SocketAddr::from(([0, 0, 0, 0], 8080));
        assert!(matches!(
            ensure_secure_bind(public, false, false, false),
            Err(ServerError::InsecureBind { .. })
        ));
        assert!(matches!(
            ensure_secure_bind(public, true, false, true),
            Err(ServerError::InsecureBind { .. })
        ));
        assert!(matches!(
            ensure_secure_bind(public, false, true, true),
            Err(ServerError::InsecureBind { .. })
        ));
        assert!(matches!(
            ensure_secure_bind(public, true, true, false),
            Err(ServerError::InsecureBind { .. })
        ));
        assert!(ensure_secure_bind(public, true, true, true).is_ok());
    }
}
