//! Server health integration tests.

mod common;

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use kira_protocol::HealthResponse;
use kira_server::router;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() -> Result<(), Box<dyn Error>> {
    let res = router(common::empty_state())
        .oneshot(Request::builder().uri("/health").body(Body::empty())?)
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await?.to_bytes();
    let health: HealthResponse = serde_json::from_slice(&bytes)?;
    assert_eq!(health.status, "ok");
    Ok(())
}

#[tokio::test]
async fn unknown_path_is_404() -> Result<(), Box<dyn Error>> {
    let res = router(common::empty_state())
        .oneshot(Request::builder().uri("/nope").body(Body::empty())?)
        .await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    Ok(())
}
