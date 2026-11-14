//! Report API integration tests.

mod common;

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use kira_protocol::{Report, ReportList};
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
