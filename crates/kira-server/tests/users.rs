//! Guild user API integration tests.

mod common;

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use kira_protocol::GuildUserList;
use kira_server::{AppState, router};
use kira_store::Database;
use tower::ServiceExt;

async fn state() -> Result<(AppState, Database), Box<dyn Error>> {
    let database = common::database().await;
    Ok((common::state_with_database(database.clone()), database))
}

async fn list(state: &AppState, uri: &str, cookie: &str) -> Result<GuildUserList, Box<dyn Error>> {
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
async fn users_require_a_session() -> Result<(), Box<dyn Error>> {
    let (state, _database) = state().await?;
    let response = router(state)
        .oneshot(Request::builder().uri("/guild/users").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn users_are_listed_searched_filtered_and_paginated() -> Result<(), Box<dyn Error>> {
    let (state, database) = state().await?;
    let cookie = common::login_cookie(&state).await?;
    let users = database.users();

    users.join("1", "alice", "Alice").await?;
    users.join("2", "bob", "Bobby").await?;
    users.leave("2", "bob", "Bobby").await?;

    let page = list(&state, "/guild/users?page=1&page_limit=1", &cookie).await?;
    assert_eq!(page.total, 2);
    assert_eq!(page.page, 1);
    assert_eq!(page.page_limit, 1);
    assert_eq!(page.users.len(), 1);

    let second = list(&state, "/guild/users?page=2&page_limit=1", &cookie).await?;
    assert_eq!(second.users.len(), 1);

    let searched = list(&state, "/guild/users?search=bobby", &cookie).await?;
    assert_eq!(searched.total, 1);
    assert_eq!(searched.users[0].user_id, "2");

    let active = list(&state, "/guild/users?status=active", &cookie).await?;
    assert_eq!(active.total, 1);
    assert_eq!(active.users[0].user_id, "1");

    let left = list(&state, "/guild/users?status=left", &cookie).await?;
    assert_eq!(left.total, 1);
    assert!(left.users[0].leave_at.is_some());
    Ok(())
}

#[tokio::test]
async fn unknown_status_filters_are_rejected() -> Result<(), Box<dyn Error>> {
    let (state, _database) = state().await?;
    let cookie = common::login_cookie(&state).await?;

    let response = router(state)
        .oneshot(common::with_cookie(
            Request::builder()
                .uri("/guild/users?status=banned")
                .body(Body::empty())?,
            &cookie,
        ))
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
