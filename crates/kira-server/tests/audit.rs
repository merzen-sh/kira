//! Audit log API integration tests.

mod common;

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use kira_protocol::{AuditEntity, AuditEventList, AuditFacets};
use kira_server::{AppState, router};
use kira_store::Database;
use kira_store::audit::NewAuditEvent;
use serde_json::Map;
use tower::ServiceExt;

async fn state() -> Result<(AppState, Database), Box<dyn Error>> {
    let database = common::database().await;
    Ok((common::state_with_database(database.clone()), database))
}

async fn list(state: &AppState, uri: &str, cookie: &str) -> Result<AuditEventList, Box<dyn Error>> {
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

fn entity(kind: &str, id: &str, name: &str) -> AuditEntity {
    AuditEntity::new(kind, Some(id.to_owned()), Some(name.to_owned()))
}

#[tokio::test]
async fn audit_routes_require_a_session() -> Result<(), Box<dyn Error>> {
    let (state, _database) = state().await?;

    for uri in ["/audit/events", "/audit/facets"] {
        let response = router(state.clone())
            .oneshot(Request::builder().uri(uri).body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    Ok(())
}

#[tokio::test]
async fn events_are_listed_searched_filtered_and_paginated() -> Result<(), Box<dyn Error>> {
    let (state, database) = state().await?;
    let cookie = common::login_cookie(&state).await?;
    let audit = database.audit();
    let alice = entity("user", "42", "Alice");
    let system = entity("system", "scheduler", "Scheduler");
    let metadata = Map::new();

    audit
        .record(&NewAuditEvent::new(
            "report.created",
            &alice,
            None,
            &metadata,
        ))
        .await?;
    audit
        .record(&NewAuditEvent::new(
            "report.resolved",
            &alice,
            None,
            &metadata,
        ))
        .await?;
    audit
        .record(&NewAuditEvent::new(
            "system.startup",
            &system,
            None,
            &metadata,
        ))
        .await?;

    let page = list(&state, "/audit/events?page=1&page_limit=1", &cookie).await?;
    assert_eq!(page.total, 3);
    assert_eq!(page.page, 1);
    assert_eq!(page.page_limit, 1);
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].action, "system.startup");

    let second = list(&state, "/audit/events?page=2&page_limit=1", &cookie).await?;
    assert_eq!(second.events[0].action, "report.resolved");

    let searched = list(&state, "/audit/events?search=scheduler", &cookie).await?;
    assert_eq!(searched.total, 1);
    assert_eq!(searched.events[0].action, "system.startup");

    let by_action = list(&state, "/audit/events?action=report.resolved", &cookie).await?;
    assert_eq!(by_action.total, 1);
    assert_eq!(by_action.events[0].action, "report.resolved");

    let by_actor = list(&state, "/audit/events?actor_kind=system", &cookie).await?;
    assert_eq!(by_actor.total, 1);
    assert_eq!(by_actor.events[0].action, "system.startup");
    Ok(())
}

#[tokio::test]
async fn facets_list_distinct_filter_values() -> Result<(), Box<dyn Error>> {
    let (state, database) = state().await?;
    let cookie = common::login_cookie(&state).await?;
    let alice = entity("user", "42", "Alice");
    let metadata = Map::new();

    database
        .audit()
        .record(&NewAuditEvent::new(
            "report.resolved",
            &alice,
            None,
            &metadata,
        ))
        .await?;

    let response = router(state)
        .oneshot(common::with_cookie(
            Request::builder()
                .uri("/audit/facets")
                .body(Body::empty())?,
            &cookie,
        ))
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await?.to_bytes();
    let facets: AuditFacets = serde_json::from_slice(&bytes)?;
    assert_eq!(facets.actions, ["report.resolved"]);
    assert_eq!(facets.actor_kinds, ["user"]);
    Ok(())
}
