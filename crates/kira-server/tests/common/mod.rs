//! Shared helpers for server integration tests.

#![allow(dead_code)]

use std::error::Error;
use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use kira_server::{AppState, Auth, AuthConfig, router};
use kira_store::Store;
use tokio::sync::{mpsc, watch};
use tower::ServiceExt;

/// PIN configured for test servers.
pub const PIN: &str = "123456";

/// Authentication settings with a known PIN and the default origins.
pub fn auth() -> Auth {
    Auth::new(AuthConfig::with_password(PIN)).expect("test authentication")
}

/// In-memory report storage for tests.
pub async fn store() -> Store {
    Store::connect("sqlite::memory:")
        .await
        .expect("test report storage")
}

/// State with no cached guild data and the given report storage.
pub fn state_with_store(store: Store) -> AppState {
    let (_guild_sender, guilds) = watch::channel(Vec::new());
    let (_channel_sender, channels) = watch::channel(Vec::new());
    let (messages, _receiver) = mpsc::channel(1);

    AppState::new(1, guilds, channels, store, messages, auth())
}

/// State with no cached guild data.
pub async fn empty_state() -> AppState {
    state_with_store(store().await)
}

/// Submit a login request.
pub async fn login(state: &AppState, pin: &str) -> Result<Response, Box<dyn Error>> {
    Ok(router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::json!({ "pin": pin }).to_string()))?,
        )
        .await?)
}

/// Extract the `name=value` pair from a `Set-Cookie` header.
pub fn session_cookie(response: &Response) -> String {
    response.headers()[header::SET_COOKIE]
        .to_str()
        .expect("set-cookie header is text")
        .split(';')
        .next()
        .expect("set-cookie header has a name=value pair")
        .to_owned()
}

/// Log in with the test PIN and return the cookie pair.
pub async fn login_cookie(state: &AppState) -> Result<String, Box<dyn Error>> {
    let response = login(state, PIN).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    Ok(session_cookie(&response))
}

/// Attach a session cookie to a request.
pub fn with_cookie(mut request: Request<Body>, cookie: &str) -> Request<Body> {
    request
        .headers_mut()
        .insert(header::COOKIE, cookie.parse().expect("cookie header"));
    request
}

/// Attach a simulated peer address for the rate limiter.
pub fn with_peer(mut request: Request<Body>, peer: SocketAddr) -> Request<Body> {
    request.extensions_mut().insert(ConnectInfo(peer));
    request
}
