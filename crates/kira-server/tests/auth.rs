//! PIN authentication, cookie, CORS, and expiration tests.

mod common;

use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use kira_server::{AppState, Auth, AuthConfig, COOKIE_NAME, SESSION_TTL, router};
use tokio::sync::{mpsc, watch};
use tokio::time::advance;
use tower::ServiceExt;

use common::{PIN, empty_state, login, login_cookie, session_cookie, with_cookie, with_peer};

const ALLOWED_ORIGIN: &str = "http://127.0.0.1:3000";

async fn state() -> AppState {
    empty_state().await
}

fn session_request(cookie: Option<&str>) -> Request<Body> {
    let request = Request::builder()
        .uri("/auth/session")
        .body(Body::empty())
        .expect("request");

    match cookie {
        Some(cookie) => with_cookie(request, cookie),
        None => request,
    }
}

#[tokio::test]
async fn login_sets_a_hardened_cookie() -> Result<(), Box<dyn Error>> {
    let state = state().await;
    let response = login(&state, PIN).await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let cookie = response.headers()[header::SET_COOKIE].to_str()?;
    assert!(cookie.starts_with(&format!("{COOKIE_NAME}=")));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("Path=/"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(cookie.contains("Max-Age=43200"));
    assert!(!cookie.contains("Secure"), "plain HTTP must not set Secure");
    assert!(
        !cookie.contains(PIN),
        "the PIN must never appear in a cookie"
    );
    Ok(())
}

#[tokio::test]
async fn a_wrong_pin_is_rejected_without_a_cookie() -> Result<(), Box<dyn Error>> {
    let response = login(&state().await, "000000").await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    Ok(())
}

#[tokio::test]
async fn a_malformed_login_body_is_rejected() -> Result<(), Box<dyn Error>> {
    let response = router(state().await)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"nope":true}"#))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    Ok(())
}

#[tokio::test]
async fn session_requires_a_valid_cookie() -> Result<(), Box<dyn Error>> {
    let state = state().await;
    let response = router(state.clone()).oneshot(session_request(None)).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let cookie = login_cookie(&state).await?;
    let response = router(state)
        .oneshot(session_request(Some(&cookie)))
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    Ok(())
}

#[tokio::test]
async fn protected_routes_require_a_session() -> Result<(), Box<dyn Error>> {
    let state = state().await;

    let guild = router(state.clone())
        .oneshot(Request::builder().uri("/guild").body(Body::empty())?)
        .await?;
    assert_eq!(guild.status(), StatusCode::UNAUTHORIZED);

    let message = router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/guild/channels/10/messages")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"content":"Hello"}"#))?,
        )
        .await?;
    assert_eq!(message.status(), StatusCode::UNAUTHORIZED);

    let cookie = login_cookie(&state).await?;
    let guild = router(state)
        .oneshot(with_cookie(
            Request::builder().uri("/guild").body(Body::empty())?,
            &cookie,
        ))
        .await?;

    // Authenticated, but the guild cache is empty.
    assert_eq!(guild.status(), StatusCode::SERVICE_UNAVAILABLE);
    Ok(())
}

#[tokio::test]
async fn bearer_tokens_are_not_accepted() -> Result<(), Box<dyn Error>> {
    let response = router(state().await)
        .oneshot(
            Request::builder()
                .uri("/guild")
                .header(header::AUTHORIZATION, "Bearer secret")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn tampered_tokens_are_rejected() -> Result<(), Box<dyn Error>> {
    let state = state().await;
    let cookie = login_cookie(&state).await?;
    let (name, token) = cookie.split_once('=').ok_or("cookie pair")?;
    let tampered = format!("{name}={token}0");

    let response = router(state)
        .oneshot(session_request(Some(&tampered)))
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn expired_tokens_are_rejected() -> Result<(), Box<dyn Error>> {
    // Pause after building the state: opening the report store must not race
    // the paused clock's auto-advance.
    let state = state().await;
    tokio::time::pause();
    let cookie = login_cookie(&state).await?;

    advance(SESSION_TTL + Duration::from_secs(1)).await;

    let response = router(state)
        .oneshot(session_request(Some(&cookie)))
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn logout_clears_the_cookie() -> Result<(), Box<dyn Error>> {
    let response = router(state().await)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/logout")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let cookie = response.headers()[header::SET_COOKIE].to_str()?;
    assert!(cookie.starts_with(&format!("{COOKIE_NAME}=;")));
    assert!(cookie.contains("Max-Age=0"));
    assert!(cookie.contains("HttpOnly"));
    Ok(())
}

#[tokio::test]
async fn login_is_rate_limited_per_peer() -> Result<(), Box<dyn Error>> {
    // Pause after building the state: opening the report store must not race
    // the paused clock's auto-advance.
    let state = state().await;
    tokio::time::pause();
    let peer = SocketAddr::from(([192, 0, 2, 10], 4000));

    for _ in 0..5 {
        let response = router(state.clone())
            .oneshot(with_peer(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"pin":"000000"}"#))?,
                peer,
            ))
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let locked = router(state.clone())
        .oneshot(with_peer(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"pin":"{PIN}"}}"#)))?,
            peer,
        ))
        .await?;
    assert_eq!(locked.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(locked.headers().get(header::RETRY_AFTER).is_some());

    // A different peer is unaffected.
    let other = login(&state, PIN).await?;
    assert_eq!(other.status(), StatusCode::NO_CONTENT);

    advance(Duration::from_secs(15 * 60 + 1)).await;
    let unlocked = router(state)
        .oneshot(with_peer(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"pin":"{PIN}"}}"#)))?,
            peer,
        ))
        .await?;
    assert_eq!(unlocked.status(), StatusCode::NO_CONTENT);
    Ok(())
}

#[tokio::test]
async fn unknown_origins_are_rejected() -> Result<(), Box<dyn Error>> {
    let response = router(state().await)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ORIGIN, "http://evil.example")
                .body(Body::from(format!(r#"{{"pin":"{PIN}"}}"#)))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
    Ok(())
}

#[tokio::test]
async fn configured_origins_are_accepted() -> Result<(), Box<dyn Error>> {
    let response = router(state().await)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ORIGIN, ALLOWED_ORIGIN)
                .body(Body::from(format!(r#"{{"pin":"{PIN}"}}"#)))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        ALLOWED_ORIGIN
    );
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_CREDENTIALS],
        "true"
    );
    Ok(())
}

#[tokio::test]
async fn cors_preflight_reflects_allowed_origins() -> Result<(), Box<dyn Error>> {
    let response = router(state().await)
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/guild")
                .header(header::ORIGIN, ALLOWED_ORIGIN)
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())?,
        )
        .await?;

    assert!(response.status().is_success());
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        ALLOWED_ORIGIN
    );
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_CREDENTIALS],
        "true"
    );

    let denied = router(empty_state().await)
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/guild")
                .header(header::ORIGIN, "http://evil.example")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(Body::empty())?,
        )
        .await?;

    assert!(
        denied
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none()
    );
    Ok(())
}

#[tokio::test]
async fn secure_cookies_are_marked_secure_when_configured() -> Result<(), Box<dyn Error>> {
    let auth = Auth::new(AuthConfig {
        cookie_secure: true,
        ..AuthConfig::with_password(PIN)
    })?;
    let (_guild_sender, guilds) = watch::channel(Vec::new());
    let (_channel_sender, channels) = watch::channel(Vec::new());
    let (messages, _receiver) = mpsc::channel(1);
    let state = AppState::new(1, guilds, channels, common::store().await, messages, auth);

    let response = login(&state, PIN).await?;
    let cookie = session_cookie(&response);
    let header = response.headers()[header::SET_COOKIE].to_str()?;

    assert!(cookie.starts_with(&format!("{COOKIE_NAME}=")));
    assert!(header.contains("Secure"));
    Ok(())
}

#[tokio::test]
async fn same_site_none_requires_secure() -> Result<(), Box<dyn Error>> {
    let config = kira_config::ServerConfig::try_from_iter([
        ("KIRA_GUILD_ID", "1"),
        ("KIRA_COOKIE_SAMESITE", "none"),
    ])?;

    assert!(matches!(
        AuthConfig::from_server_config(&config),
        Err(kira_server::ServerError::InsecureCookie)
    ));

    let config = kira_config::ServerConfig::try_from_iter([
        ("KIRA_GUILD_ID", "1"),
        ("KIRA_COOKIE_SAMESITE", "none"),
        ("KIRA_COOKIE_SECURE", "true"),
    ])?;
    assert!(AuthConfig::from_server_config(&config).is_ok());
    Ok(())
}

#[tokio::test]
async fn health_stays_public() -> Result<(), Box<dyn Error>> {
    let response = router(empty_state().await)
        .oneshot(Request::builder().uri("/health").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn generated_pins_are_printed_once() -> Result<(), Box<dyn Error>> {
    let auth = Auth::new(AuthConfig::default())?;
    let pin = auth.generated_pin().ok_or("generated PIN missing")?;

    assert_eq!(pin.len(), 6);
    assert!(pin.chars().all(|character| character.is_ascii_digit()));
    Ok(())
}
