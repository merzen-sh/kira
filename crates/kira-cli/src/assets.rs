//! Embedded dashboard asset lookup and response headers.

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

use crate::AppState;

#[derive(RustEmbed)]
#[folder = "../../apps/dashboard/dist/"]
struct Dashboard;

/// Look up an embedded dashboard file by its exact relative path.
#[must_use]
pub fn get(path: &str) -> Option<rust_embed::EmbeddedFile> {
    Dashboard::get(path)
}

/// Pick a `Content-Type` from the file extension.
#[must_use]
pub fn content_type_for(path: &str) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string()
}

/// Pick a `Cache-Control` policy for a dashboard file.
#[must_use]
pub fn cache_control_for(path: &str) -> &'static str {
    if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    }
}

/// Build a response for an embedded asset, or 404 when it does not exist.
pub(crate) fn serve(path: &str) -> Response {
    let Some(file) = get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let content_type = match header::HeaderValue::from_str(&content_type_for(path)) {
        Ok(value) => value,
        Err(error) => {
            tracing::error!("invalid content type for {path}: {error}");
            header::HeaderValue::from_static("application/octet-stream")
        }
    };

    (
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CACHE_CONTROL,
                header::HeaderValue::from_static(cache_control_for(path)),
            ),
        ],
        Body::from(file.data.into_owned()),
    )
        .into_response()
}

/// Serve the SPA for non-API paths and never fall back to `index.html` for
/// API routes. In development, non-API paths are proxied to Vite instead.
pub(crate) async fn fallback(State(state): State<AppState>, request: Request) -> Response {
    let path = request.uri().path().trim_start_matches('/');

    if path == "api" || path.starts_with("api/") {
        return StatusCode::NOT_FOUND.into_response();
    }

    if let Some(base_url) = state.dev_server_url.as_deref() {
        return crate::dev_proxy::forward(base_url, request).await;
    }

    embedded(request.uri())
}

fn embedded(uri: &Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if path.starts_with("assets/") {
        if !is_safe_path(path) {
            return StatusCode::NOT_FOUND.into_response();
        }
        return serve(path);
    }

    serve("index.html")
}

fn is_safe_path(path: &str) -> bool {
    !path
        .split(['/', '\\'])
        .any(|segment| segment == "." || segment == "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn javascript_content_type() {
        assert!(content_type_for("app.js").contains("javascript"));
    }

    #[test]
    fn asset_cache_control_is_immutable() {
        assert_eq!(
            cache_control_for("assets/app.js"),
            "public, max-age=31536000, immutable"
        );
    }

    #[test]
    fn index_is_not_immutable() {
        assert_eq!(cache_control_for("index.html"), "no-cache");
    }

    #[test]
    fn traversal_paths_are_rejected() {
        assert!(!is_safe_path("assets/../../etc/passwd"));
        assert!(!is_safe_path("assets/./secret"));
        assert!(is_safe_path("assets/index-BV3wEoCd.js"));
    }
}
