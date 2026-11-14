//! Guild details integration tests.

mod common;

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use kira_protocol::{ErrorResponse, GuildDetails};
use kira_server::{AppState, router};
use tokio::sync::{mpsc, watch};
use tower::ServiceExt;

fn details(id: &str, name: &str) -> GuildDetails {
    let mut guild = GuildDetails::new(id, name);
    guild.description = Some("A test server".into());
    guild.icon_url = Some("https://cdn.example/icon.webp".into());
    guild.owner_id = "42".into();
    guild.member_count = 12;
    guild.presence_count = Some(3);
    guild.preferred_locale = "en-US".into();
    guild.features = vec!["COMMUNITY".into()];
    guild
}

async fn get_guild(
    state: AppState,
    cookie: &str,
) -> Result<axum::response::Response, Box<dyn Error>> {
    Ok(router(state)
        .oneshot(common::with_cookie(
            Request::builder().uri("/guild").body(Body::empty())?,
            cookie,
        ))
        .await?)
}

#[tokio::test]
async fn guild_returns_503_without_data() -> Result<(), Box<dyn Error>> {
    let state = common::empty_state().await;
    let cookie = common::login_cookie(&state).await?;
    let res = get_guild(state, &cookie).await?;

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

    let bytes = res.into_body().collect().await?.to_bytes();
    let body: ErrorResponse = serde_json::from_slice(&bytes)?;
    assert!(!body.error.is_empty());
    Ok(())
}

#[tokio::test]
async fn guild_returns_the_configured_server() -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = watch::channel(Vec::new());
    let (_channel_sender, channels) = watch::channel(Vec::new());
    let (messages, _message_receiver) = mpsc::channel(1);
    let state = AppState::new(
        1,
        receiver,
        channels,
        common::database().await,
        messages,
        common::auth(),
    );
    let cookie = common::login_cookie(&state).await?;

    sender.send_replace(vec![details("1", "Alpha"), details("2", "Beta")]);

    let res = get_guild(state, &cookie).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let content_type = res.headers()[header::CONTENT_TYPE].to_str()?;
    assert!(
        content_type.starts_with("application/json"),
        "unexpected content type: {content_type}"
    );

    let bytes = res.into_body().collect().await?.to_bytes();
    let body: GuildDetails = serde_json::from_slice(&bytes)?;
    assert_eq!(body.id, "1");
    assert_eq!(body.name, "Alpha");
    assert_eq!(body.member_count, 12);
    assert_eq!(body.features, vec!["COMMUNITY".to_string()]);
    Ok(())
}

#[tokio::test]
async fn guild_returns_503_when_the_target_is_missing() -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = watch::channel(Vec::new());
    let (_channel_sender, channels) = watch::channel(Vec::new());
    let (messages, _message_receiver) = mpsc::channel(1);
    let state = AppState::new(
        9,
        receiver,
        channels,
        common::database().await,
        messages,
        common::auth(),
    );
    let cookie = common::login_cookie(&state).await?;

    sender.send_replace(vec![details("1", "Alpha")]);

    let res = get_guild(state, &cookie).await?;
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    Ok(())
}
