//! Dashboard asset integration tests.

use anyhow::Result;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use http_body_util::BodyExt;
use kira_cli::{AppState, router};
use kira_client::Client;
use tower::ServiceExt;

fn app() -> axum::Router {
    router(AppState::new(Client::new("http://127.0.0.1:1"), None))
}

async fn get(path: &str) -> Result<Response> {
    Ok(app()
        .oneshot(Request::builder().uri(path).body(Body::empty())?)
        .await?)
}

async fn body_string(res: Response) -> Result<String> {
    let bytes = res.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn index_is_served() -> Result<()> {
    let res = get("/").await?;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "text/html");
    assert_eq!(res.headers()[header::CACHE_CONTROL], "no-cache");
    assert!(body_string(res).await?.contains("<div id=\"root\">"));
    Ok(())
}

#[tokio::test]
async fn spa_fallback_serves_index() -> Result<()> {
    let res = get("/dashboard/settings").await?;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "text/html");
    assert!(body_string(res).await?.contains("<div id=\"root\">"));
    Ok(())
}

#[tokio::test]
async fn missing_asset_is_404() -> Result<()> {
    assert_eq!(
        get("/assets/does-not-exist.js").await?.status(),
        StatusCode::NOT_FOUND
    );
    Ok(())
}

#[tokio::test]
async fn api_unknown_path_is_404() -> Result<()> {
    let res = get("/api/does-not-exist").await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert!(!body_string(res).await?.contains("<div id=\"root\">"));
    Ok(())
}

#[tokio::test]
async fn traversal_does_not_escape() -> Result<()> {
    let res = get("/../../etc/passwd").await?;
    let body = body_string(res).await?;
    assert!(!body.contains("root:"));
    Ok(())
}
