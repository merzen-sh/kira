//! Guild message integration tests.

mod common;

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use kira_protocol::{ChannelDetails, ErrorResponse, GuildDetails, SentMessage};
use kira_server::{AppState, router};
use tokio::sync::{mpsc, watch};
use tower::ServiceExt;

async fn state() -> (
    AppState,
    mpsc::Receiver<kira_core::messaging::OutgoingMessage>,
) {
    let (guild_sender, guilds) = watch::channel(Vec::new());
    let (channel_sender, channels) = watch::channel(Vec::new());
    let (messages, receiver) = mpsc::channel(1);
    let state = AppState::new(
        1,
        guilds,
        channels,
        common::database().await,
        messages,
        common::auth(),
    );

    guild_sender.send_replace(vec![GuildDetails::new("1", "Alpha")]);
    channel_sender.send_replace(vec![ChannelDetails::new("10", "1", "general", "text")]);

    (state, receiver)
}

async fn post(
    state: AppState,
    cookie: &str,
    channel_id: &str,
    body: &str,
) -> Result<axum::response::Response, Box<dyn Error>> {
    Ok(router(state)
        .oneshot(common::with_cookie(
            Request::builder()
                .method("POST")
                .uri(format!("/guild/channels/{channel_id}/messages"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_owned()))?,
            cookie,
        ))
        .await?)
}

#[tokio::test]
async fn message_is_sent_to_the_selected_channel() -> Result<(), Box<dyn Error>> {
    let (state, mut messages) = state().await;
    let cookie = common::login_cookie(&state).await?;
    tokio::spawn(async move {
        let Some(message) = messages.recv().await else {
            return;
        };
        let _ = message.reply.send(Ok("99".into()));
    });

    let res = post(state, &cookie, "10", r#"{"content":"Hello"}"#).await?;
    assert_eq!(res.status(), StatusCode::CREATED);

    let bytes = res.into_body().collect().await?.to_bytes();
    let message: SentMessage = serde_json::from_slice(&bytes)?;
    assert_eq!(message.id, "99");
    assert_eq!(message.channel_id, "10");
    Ok(())
}

#[tokio::test]
async fn raw_message_is_forwarded() -> Result<(), Box<dyn Error>> {
    let (state, mut messages) = state().await;
    let cookie = common::login_cookie(&state).await?;
    let handle = tokio::spawn(async move {
        let message = messages.recv().await?;
        let _ = message.reply.send(Ok("99".into()));
        Some(message.payload)
    });
    let body = r#"{"content":"Hey","embeds":[{"title":"Update"}],"attachments":[]}"#;

    let res = post(state, &cookie, "10", body).await?;
    assert_eq!(res.status(), StatusCode::CREATED);

    let payload = handle.await?;
    assert_eq!(
        payload.as_ref(),
        Some(&serde_json::from_str::<serde_json::Value>(body)?)
    );
    Ok(())
}

#[tokio::test]
async fn empty_message_is_rejected() -> Result<(), Box<dyn Error>> {
    let (state, _messages) = state().await;
    let cookie = common::login_cookie(&state).await?;

    let res = post(state, &cookie, "10", "{}").await?;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let bytes = res.into_body().collect().await?.to_bytes();
    let error: ErrorResponse = serde_json::from_slice(&bytes)?;
    assert_eq!(error.error, "content or embeds is required");
    Ok(())
}

#[tokio::test]
async fn unknown_channel_is_rejected() -> Result<(), Box<dyn Error>> {
    let (state, _messages) = state().await;
    let cookie = common::login_cookie(&state).await?;

    let res = post(state, &cookie, "11", r#"{"content":"Hello"}"#).await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn non_text_channel_is_rejected() -> Result<(), Box<dyn Error>> {
    let (guild_sender, guilds) = watch::channel(Vec::new());
    let (channel_sender, channels) = watch::channel(Vec::new());
    let (messages, _message_receiver) = mpsc::channel(1);
    let state = AppState::new(
        1,
        guilds,
        channels,
        common::database().await,
        messages,
        common::auth(),
    );
    let cookie = common::login_cookie(&state).await?;
    guild_sender.send_replace(vec![GuildDetails::new("1", "Alpha")]);
    channel_sender.send_replace(vec![ChannelDetails::new("10", "1", "voice", "voice")]);

    let res = post(state, &cookie, "10", r#"{"content":"Hello"}"#).await?;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn content_over_the_limit_is_rejected() -> Result<(), Box<dyn Error>> {
    let (state, _messages) = state().await;
    let cookie = common::login_cookie(&state).await?;
    let body = serde_json::json!({ "content": "x".repeat(2001) }).to_string();

    let res = post(state, &cookie, "10", &body).await?;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn embeds_only_payload_is_accepted() -> Result<(), Box<dyn Error>> {
    let (state, mut messages) = state().await;
    let cookie = common::login_cookie(&state).await?;
    tokio::spawn(async move {
        if let Some(message) = messages.recv().await {
            let _ = message.reply.send(Ok("99".into()));
        }
    });

    let res = post(state, &cookie, "10", r#"{"embeds":[{"title":"x"}]}"#).await?;
    assert_eq!(res.status(), StatusCode::CREATED);
    Ok(())
}

#[tokio::test]
async fn stalled_delivery_times_out() -> Result<(), Box<dyn Error>> {
    let (state, mut messages) = state().await;
    let cookie = common::login_cookie(&state).await?;
    // Pause after building the state: opening the report store must not race
    // the paused clock's auto-advance.
    tokio::time::pause();
    let _hold = tokio::spawn(async move {
        if let Some(message) = messages.recv().await {
            let _reply = message.reply;
            std::future::pending::<()>().await;
        }
    });

    let res = post(state, &cookie, "10", r#"{"content":"Hello"}"#).await?;
    assert_eq!(res.status(), StatusCode::GATEWAY_TIMEOUT);
    Ok(())
}
