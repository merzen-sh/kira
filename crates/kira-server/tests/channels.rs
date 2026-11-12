//! Guild channels integration tests.

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use kira_protocol::{ChannelDetails, ErrorResponse, GuildDetails};
use kira_server::{AppState, router};
use tokio::sync::{mpsc, watch};
use tower::ServiceExt;

async fn get_channels(state: AppState) -> Result<axum::response::Response, Box<dyn Error>> {
    Ok(router(state)
        .oneshot(
            Request::builder()
                .uri("/guild/channels")
                .body(Body::empty())?,
        )
        .await?)
}

#[tokio::test]
async fn channels_return_503_without_data() -> Result<(), Box<dyn Error>> {
    let res = get_channels(AppState::default()).await?;

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

    let bytes = res.into_body().collect().await?.to_bytes();
    let body: ErrorResponse = serde_json::from_slice(&bytes)?;
    assert!(!body.error.is_empty());
    Ok(())
}

#[tokio::test]
async fn channels_return_the_configured_server_channels() -> Result<(), Box<dyn Error>> {
    let (guild_sender, guilds) = watch::channel(Vec::new());
    let (channel_sender, channels) = watch::channel(Vec::new());
    let (messages, _message_receiver) = mpsc::channel(1);
    let state = AppState::new(Some(1), guilds, channels, messages);

    guild_sender.send_replace(vec![GuildDetails::new("1", "Alpha")]);
    channel_sender.send_replace(vec![
        ChannelDetails::new("10", "1", "general", "text"),
        ChannelDetails::new("11", "2", "other", "text"),
    ]);

    let res = get_channels(state).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = res.into_body().collect().await?.to_bytes();
    let body: Vec<ChannelDetails> = serde_json::from_slice(&bytes)?;
    assert_eq!(body.len(), 1);
    assert_eq!(body[0].id, "10");
    assert_eq!(body[0].name, "general");
    Ok(())
}
