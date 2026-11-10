//! Development-only reverse proxy to the Vite dev server.

use anyhow::Result;
use axum::Json;
use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use kira_protocol::ErrorResponse;

const MAX_BODY_SIZE: usize = 16 * 1024 * 1024;

/// Forward a non-API request to the Vite dev server.
pub(crate) async fn forward(base_url: &str, request: Request) -> Response {
    match forward_inner(base_url, request).await {
        Ok(response) => response,
        Err(error) => {
            eprintln!("vite dev proxy failed: {error:#}");
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new("vite dev server unavailable")),
            )
                .into_response()
        }
    }
}

async fn forward_inner(base_url: &str, request: Request) -> Result<Response> {
    let (parts, body) = request.into_parts();
    let path = parts
        .uri
        .path_and_query()
        .map(|path| path.as_str())
        .unwrap_or("/");
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let body = to_bytes(body, MAX_BODY_SIZE).await?;
    let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())?;

    let response = reqwest::Client::new()
        .request(method, url)
        .headers(sanitize_headers(parts.headers))
        .body(body)
        .send()
        .await?;

    let status = response.status();
    let headers = sanitize_headers(response.headers().clone());
    let body = response.bytes().await?;

    let mut result = Response::new(Body::from(body));
    *result.status_mut() = status;
    *result.headers_mut() = headers;
    Ok(result)
}

fn sanitize_headers(mut headers: HeaderMap) -> HeaderMap {
    for name in [
        "connection",
        "content-length",
        "host",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
    ] {
        headers.remove(name);
    }
    headers
}
