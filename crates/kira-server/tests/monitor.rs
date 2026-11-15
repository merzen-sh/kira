//! System monitor API integration tests.

mod common;

use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use kira_protocol::{MonitorReport, MonitorSample};
use kira_server::{AppState, router};
use kira_store::Database;
use tower::ServiceExt;

async fn state() -> Result<(AppState, Database), Box<dyn Error>> {
    let store = common::database().await;
    Ok((common::state_with_database(store.clone()), store))
}

fn now() -> Result<i64, Box<dyn Error>> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(i64::try_from(secs)?)
}

#[tokio::test]
async fn monitor_requires_a_session() -> Result<(), Box<dyn Error>> {
    let (state, _store) = state().await?;
    let response = router(state)
        .oneshot(Request::builder().uri("/monitor").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn monitor_returns_current_usage_and_history() -> Result<(), Box<dyn Error>> {
    let (state, store) = state().await?;
    let cookie = common::login_cookie(&state).await?;
    let now = now()?;

    for (offset, cpu) in [(60, 10.0), (30, 20.0), (0, 30.0)] {
        store
            .monitor()
            .record(&MonitorSample::new(now - offset, cpu, 100, 200, 300, 400))
            .await?;
    }

    let response = router(state)
        .oneshot(common::with_cookie(
            Request::builder()
                .uri("/monitor?range=1h")
                .body(Body::empty())?,
            &cookie,
        ))
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await?.to_bytes();
    let report: MonitorReport = serde_json::from_slice(&bytes)?;

    assert_eq!(report.range, "1h");
    assert_eq!(report.current.expect("current sample").recorded_at, now);
    assert!(!report.history.is_empty());
    assert!(
        report
            .history
            .iter()
            .all(|sample| sample.recorded_at >= now - 60)
    );
    Ok(())
}

#[tokio::test]
async fn monitor_rejects_an_unknown_range() -> Result<(), Box<dyn Error>> {
    let (state, _store) = state().await?;
    let cookie = common::login_cookie(&state).await?;

    let response = router(state)
        .oneshot(common::with_cookie(
            Request::builder()
                .uri("/monitor?range=week")
                .body(Body::empty())?,
            &cookie,
        ))
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
