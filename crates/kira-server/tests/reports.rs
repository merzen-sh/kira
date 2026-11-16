//! Report API integration tests.

mod common;

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use kira_protocol::{Report, ReportList, ReportStats};
use kira_server::{AppState, router};
use kira_store::Database;
use kira_store::reports::NewReport;
use tower::ServiceExt;

async fn state() -> Result<(AppState, Database), Box<dyn Error>> {
    let store = common::database().await;
    Ok((common::state_with_database(store.clone()), store))
}

async fn list(state: &AppState, uri: &str, cookie: &str) -> Result<ReportList, Box<dyn Error>> {
    let response = router(state.clone())
        .oneshot(common::with_cookie(
            Request::builder().uri(uri).body(Body::empty())?,
            cookie,
        ))
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(serde_json::from_slice(&bytes)?)
}

#[tokio::test]
async fn reports_require_a_session() -> Result<(), Box<dyn Error>> {
    let (state, _store) = state().await?;
    let response = router(state)
        .oneshot(Request::builder().uri("/reports").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn reports_are_listed_searched_and_paginated() -> Result<(), Box<dyn Error>> {
    let (state, store) = state().await?;
    let cookie = common::login_cookie(&state).await?;

    store
        .reports()
        .create_report(&NewReport::new(
            "42",
            "Login broken",
            "open the dashboard",
            "the PIN is rejected",
        ))
        .await?;
    store
        .reports()
        .create_report(&NewReport::new(
            "7",
            "Channel list empty",
            "open announcements",
            "no channels are shown",
        ))
        .await?;

    let page = list(&state, "/reports?page=1&page_limit=1", &cookie).await?;
    assert_eq!(page.total, 2);
    assert_eq!(page.page, 1);
    assert_eq!(page.page_limit, 1);
    assert_eq!(page.reports.len(), 1);
    assert_eq!(page.reports[0].title, "Channel list empty");

    let second = list(&state, "/reports?page=2&page_limit=1", &cookie).await?;
    assert_eq!(second.reports[0].title, "Login broken");

    let searched = list(&state, "/reports?search=pin", &cookie).await?;
    assert_eq!(searched.total, 1);
    assert_eq!(searched.reports[0].title, "Login broken");
    Ok(())
}

#[tokio::test]
async fn report_stats_count_open_and_resolved() -> Result<(), Box<dyn Error>> {
    let (state, store) = state().await?;
    let cookie = common::login_cookie(&state).await?;

    let first = store
        .reports()
        .create_report(&NewReport::new("42", "Login broken", "open", "rejected"))
        .await?;
    store
        .reports()
        .create_report(&NewReport::new("7", "Crash", "click", "boom"))
        .await?;
    store.reports().set_resolved(first.id, true).await?;

    let response = router(state)
        .oneshot(common::with_cookie(
            Request::builder()
                .uri("/reports/stats")
                .body(Body::empty())?,
            &cookie,
        ))
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await?.to_bytes();
    let stats: ReportStats = serde_json::from_slice(&bytes)?;
    assert_eq!((stats.total, stats.resolved, stats.open), (2, 1, 1));
    Ok(())
}

#[tokio::test]
async fn reports_can_be_resolved_and_deleted() -> Result<(), Box<dyn Error>> {
    let (state, store) = state().await?;
    let cookie = common::login_cookie(&state).await?;
    let report = store
        .reports()
        .create_report(&NewReport::new("42", "Crash", "click", "boom"))
        .await?;

    let response = router(state.clone())
        .oneshot(common::with_cookie(
            Request::builder()
                .method("PATCH")
                .uri(format!("/reports/{}", report.id))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"resolved":true}"#))?,
            &cookie,
        ))
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await?.to_bytes();
    let body: Report = serde_json::from_slice(&bytes)?;
    assert!(body.resolved);

    let deleted = router(state.clone())
        .oneshot(common::with_cookie(
            Request::builder()
                .method("DELETE")
                .uri(format!("/reports/{}", report.id))
                .body(Body::empty())?,
            &cookie,
        ))
        .await?;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let missing = router(state)
        .oneshot(common::with_cookie(
            Request::builder()
                .method("DELETE")
                .uri(format!("/reports/{}", report.id))
                .body(Body::empty())?,
            &cookie,
        ))
        .await?;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn report_mutations_record_audit_events() -> Result<(), Box<dyn Error>> {
    let (state, store) = state().await?;
    let cookie = common::login_cookie(&state).await?;
    let report = store
        .reports()
        .create_report(&NewReport::new("42", "Crash", "click", "boom"))
        .await?;
    let report_id = report.id.to_string();

    let resolved = router(state.clone())
        .oneshot(common::with_cookie(
            Request::builder()
                .method("PATCH")
                .uri(format!("/reports/{report_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"resolved":true}"#))?,
            &cookie,
        ))
        .await?;
    assert_eq!(resolved.status(), StatusCode::OK);

    let unchanged = router(state.clone())
        .oneshot(common::with_cookie(
            Request::builder()
                .method("PATCH")
                .uri(format!("/reports/{report_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"resolved":true}"#))?,
            &cookie,
        ))
        .await?;
    assert_eq!(unchanged.status(), StatusCode::OK);

    let reopened = router(state.clone())
        .oneshot(common::with_cookie(
            Request::builder()
                .method("PATCH")
                .uri(format!("/reports/{report_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"resolved":false}"#))?,
            &cookie,
        ))
        .await?;
    assert_eq!(reopened.status(), StatusCode::OK);

    let deleted = router(state.clone())
        .oneshot(common::with_cookie(
            Request::builder()
                .method("DELETE")
                .uri(format!("/reports/{report_id}"))
                .body(Body::empty())?,
            &cookie,
        ))
        .await?;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let missing = router(state)
        .oneshot(common::with_cookie(
            Request::builder()
                .method("DELETE")
                .uri(format!("/reports/{report_id}"))
                .body(Body::empty())?,
            &cookie,
        ))
        .await?;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    let events = store
        .audit()
        .list_events(1, 20, None, None, None, None)
        .await?;

    assert_eq!(events.total, 4);
    let report_events = events
        .events
        .iter()
        .filter(|event| event.action.starts_with("report."))
        .collect::<Vec<_>>();
    assert_eq!(report_events.len(), 3);
    assert_eq!(report_events[0].action, "report.deleted");
    assert_eq!(report_events[1].action, "report.reopened");
    assert_eq!(report_events[2].action, "report.resolved");

    for event in report_events {
        assert_eq!(event.source, "dashboard");
        assert_eq!(event.actor.kind, "admin");
        assert_eq!(
            event
                .target
                .as_ref()
                .and_then(|target| target.id.as_deref()),
            Some(report_id.as_str())
        );
        assert_eq!(
            event
                .target
                .as_ref()
                .and_then(|target| target.name.as_deref()),
            Some("Crash")
        );
        assert!(event.metadata.is_empty());
    }
    Ok(())
}
