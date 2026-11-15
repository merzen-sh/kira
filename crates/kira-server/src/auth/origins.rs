//! Dashboard origin validation and defaults.

use std::net::SocketAddr;

use crate::errors::ServerError;

/// Vite dev server port used during local frontend development.
const VITE_DEV_PORT: u16 = 5173;

/// Default loopback dashboard address when no CLI bind is configured.
pub(super) fn default_cli_bind() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 3000))
}

pub(super) fn default_origins(cli_bind: SocketAddr) -> Vec<String> {
    let port = cli_bind.port();
    let mut origins = vec![
        format!("http://127.0.0.1:{port}"),
        format!("http://localhost:{port}"),
    ];

    if cli_bind.ip().is_loopback() && port != VITE_DEV_PORT {
        origins.push(format!("http://127.0.0.1:{VITE_DEV_PORT}"));
        origins.push(format!("http://localhost:{VITE_DEV_PORT}"));
    }

    origins
}

pub(super) fn parse_origins(value: &str) -> Result<Vec<String>, ServerError> {
    let mut origins = Vec::new();

    for raw in value.split(',') {
        let origin = raw.trim().trim_end_matches('/');
        if origin.is_empty() {
            continue;
        }
        if !is_origin(origin) {
            return Err(ServerError::InvalidOrigin);
        }
        origins.push(origin.to_owned());
    }

    if origins.is_empty() {
        return Err(ServerError::InvalidOrigin);
    }

    Ok(origins)
}

fn is_origin(value: &str) -> bool {
    let Ok(uri) = value.parse::<axum::http::Uri>() else {
        return false;
    };

    matches!(uri.scheme_str(), Some("http" | "https"))
        && uri.authority().is_some()
        && matches!(uri.path(), "" | "/")
        && uri.query().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_origins_are_rejected() {
        assert!(parse_origins("http://127.0.0.1:3000").is_ok());
        assert!(parse_origins("https://kira.example.com/").is_ok());
        assert!(parse_origins("*").is_err());
        assert!(parse_origins("http://").is_err());
        assert!(parse_origins("http://localhost:3000/path").is_err());
    }

    #[test]
    fn default_origins_follow_the_configured_cli_port() {
        let origins = default_origins(SocketAddr::from(([127, 0, 0, 1], 4000)));

        assert!(origins.contains(&"http://127.0.0.1:4000".to_owned()));
        assert!(origins.contains(&"http://localhost:4000".to_owned()));
        assert!(!origins.iter().any(|origin| origin.contains(":3000")));
        assert!(origins.contains(&format!("http://127.0.0.1:{VITE_DEV_PORT}")));
    }
}
