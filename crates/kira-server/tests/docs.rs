//! Debug-mode API reference integration tests.

#![cfg(debug_assertions)]

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use kira_server::{AppState, router};
use tower::ServiceExt;

#[tokio::test]
async fn docs_serves_scalar_reference_ui() -> Result<(), Box<dyn Error>> {
    let res = router(AppState::default())
        .oneshot(Request::builder().uri("/docs").body(Body::empty())?)
        .await?;

    assert_eq!(res.status(), StatusCode::OK);

    let content_type = res.headers()[header::CONTENT_TYPE].to_str()?;
    assert!(
        content_type.starts_with("text/html"),
        "unexpected content type: {content_type}"
    );

    let bytes = res.into_body().collect().await?.to_bytes();
    let html = String::from_utf8(bytes.to_vec())?;
    assert!(
        html.contains("id=\"api-reference\""),
        "Scalar mount point missing"
    );
    assert!(
        html.contains("data-url=\"/openapi.json\""),
        "live spec URL missing"
    );
    Ok(())
}

#[tokio::test]
async fn openapi_json_serves_the_live_spec() -> Result<(), Box<dyn Error>> {
    let res = router(AppState::default())
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(res.status(), StatusCode::OK);

    let content_type = res.headers()[header::CONTENT_TYPE].to_str()?;
    assert!(
        content_type.starts_with("application/json"),
        "unexpected content type: {content_type}"
    );

    let bytes = res.into_body().collect().await?.to_bytes();
    let spec: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(spec["info"]["title"], "Kira API");
    assert!(spec["paths"]["/health"].is_object());
    assert!(spec["paths"]["/guild"].is_object());
    assert!(spec["paths"]["/guild/channels"].is_object());
    Ok(())
}
