//! Release-mode API reference exclusion tests.

#![cfg(not(debug_assertions))]

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use kira_server::{AppState, router};
use tower::ServiceExt;

#[tokio::test]
async fn docs_routes_are_absent_outside_debug_builds() -> Result<(), Box<dyn Error>> {
    for path in ["/docs", "/openapi.json"] {
        let res = router(AppState::default())
            .oneshot(Request::builder().uri(path).body(Body::empty())?)
            .await?;
        assert_eq!(
            res.status(),
            StatusCode::NOT_FOUND,
            "{path} should not be served outside debug builds"
        );
    }
    Ok(())
}
