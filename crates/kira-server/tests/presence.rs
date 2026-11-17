//! Bot presence integration tests.

mod common;

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use kira_protocol::{ActivityKind, BotActivity, BotPresence, BotStatus};
use kira_server::{AppState, router};
use kira_store::Database;
use tokio::sync::{mpsc, watch};
use tower::ServiceExt;

async fn state_with_presence() -> (AppState, watch::Receiver<BotPresence>, Database) {
    let database = common::database().await;
    let (sender, receiver) = watch::channel(BotPresence::default());
    let (_guild_sender, guilds) = watch::channel(Vec::new());
    let (_channel_sender, channels) = watch::channel(Vec::new());
    let (messages, _message_receiver) = mpsc::channel(1);
    let state = AppState::new(
        1,
        guilds,
        channels,
        database.clone(),
        messages,
        sender,
        common::auth(),
    );

    (state, receiver, database)
}

async fn get_presence(
    state: &AppState,
    cookie: &str,
) -> Result<axum::response::Response, Box<dyn Error>> {
    Ok(router(state.clone())
        .oneshot(common::with_cookie(
            Request::builder()
                .uri("/bot/presence")
                .body(Body::empty())?,
            cookie,
        ))
        .await?)
}

async fn put_presence(
    state: &AppState,
    cookie: &str,
    body: serde_json::Value,
) -> Result<axum::response::Response, Box<dyn Error>> {
    Ok(router(state.clone())
        .oneshot(common::with_cookie(
            Request::builder()
                .method("PUT")
                .uri("/bot/presence")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))?,
            cookie,
        ))
        .await?)
}

async fn response_body<T: serde::de::DeserializeOwned>(
    response: axum::response::Response,
) -> Result<T, Box<dyn Error>> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(serde_json::from_slice(&bytes)?)
}

#[tokio::test]
async fn presence_requires_a_session() -> Result<(), Box<dyn Error>> {
    let (state, _receiver, _database) = state_with_presence().await;
    let res = router(state)
        .oneshot(
            Request::builder()
                .uri("/bot/presence")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn default_presence_is_returned() -> Result<(), Box<dyn Error>> {
    let (state, _receiver, _database) = state_with_presence().await;
    let cookie = common::login_cookie(&state).await?;
    let res = get_presence(&state, &cookie).await?;

    assert_eq!(res.status(), StatusCode::OK);
    let presence: BotPresence = response_body(res).await?;
    assert_eq!(presence, BotPresence::default());
    Ok(())
}

#[tokio::test]
async fn put_saves_broadcasts_and_audits() -> Result<(), Box<dyn Error>> {
    let (state, receiver, database) = state_with_presence().await;
    let cookie = common::login_cookie(&state).await?;
    let expected = BotPresence::new(
        BotStatus::DoNotDisturb,
        Some(BotActivity::new(ActivityKind::Watching, "Kira dashboard")),
    );

    let res = put_presence(&state, &cookie, serde_json::to_value(&expected)?).await?;
    assert_eq!(res.status(), StatusCode::OK);
    let saved: BotPresence = response_body(res).await?;
    assert_eq!(saved, expected);
    assert_eq!(*receiver.borrow(), expected);
    assert_eq!(database.presence().get().await?, expected);

    let events = database
        .audit()
        .list_events(1, 20, None, Some("bot.presence.updated"), None, None)
        .await?;
    assert_eq!(events.total, 1);
    Ok(())
}

#[tokio::test]
async fn blank_activity_text_is_rejected() -> Result<(), Box<dyn Error>> {
    let (state, receiver, database) = state_with_presence().await;
    let cookie = common::login_cookie(&state).await?;
    let res = put_presence(
        &state,
        &cookie,
        serde_json::json!({
            "status": "online",
            "activity": { "kind": "playing", "text": "   " }
        }),
    )
    .await?;

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_eq!(*receiver.borrow(), BotPresence::default());
    assert_eq!(database.presence().get().await?, BotPresence::default());
    Ok(())
}

#[tokio::test]
async fn activity_text_limit_counts_characters() -> Result<(), Box<dyn Error>> {
    let (state, _receiver, database) = state_with_presence().await;
    let cookie = common::login_cookie(&state).await?;
    let allowed = "é".repeat(128);

    let res = put_presence(
        &state,
        &cookie,
        serde_json::json!({
            "status": "online",
            "activity": { "kind": "playing", "text": allowed }
        }),
    )
    .await?;
    assert_eq!(res.status(), StatusCode::OK);

    let res = put_presence(
        &state,
        &cookie,
        serde_json::json!({
            "status": "online",
            "activity": { "kind": "playing", "text": "é".repeat(129) }
        }),
    )
    .await?;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let stored = database.presence().get().await?;
    assert_eq!(
        stored.activity.map(|activity| activity.text),
        Some("é".repeat(128))
    );
    Ok(())
}

#[tokio::test]
async fn activity_text_is_trimmed() -> Result<(), Box<dyn Error>> {
    let (state, _receiver, database) = state_with_presence().await;
    let cookie = common::login_cookie(&state).await?;
    let res = put_presence(
        &state,
        &cookie,
        serde_json::json!({
            "status": "idle",
            "activity": { "kind": "watching", "text": "  Kira  " }
        }),
    )
    .await?;

    assert_eq!(res.status(), StatusCode::OK);
    let saved: BotPresence = response_body(res).await?;
    assert_eq!(
        saved
            .activity
            .as_ref()
            .map(|activity| activity.text.as_str()),
        Some("Kira")
    );
    assert_eq!(database.presence().get().await?, saved);
    Ok(())
}

#[tokio::test]
async fn activity_can_be_cleared() -> Result<(), Box<dyn Error>> {
    let (state, receiver, database) = state_with_presence().await;
    let cookie = common::login_cookie(&state).await?;
    let with_activity = BotPresence::new(
        BotStatus::Online,
        Some(BotActivity::new(ActivityKind::Listening, "Kira FM")),
    );

    let res = put_presence(&state, &cookie, serde_json::to_value(&with_activity)?).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let cleared = BotPresence::new(BotStatus::Online, None);
    let res = put_presence(&state, &cookie, serde_json::to_value(&cleared)?).await?;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(*receiver.borrow(), cleared);
    assert_eq!(database.presence().get().await?, cleared);
    Ok(())
}

#[tokio::test]
async fn unknown_status_is_rejected() -> Result<(), Box<dyn Error>> {
    let (state, _receiver, database) = state_with_presence().await;
    let cookie = common::login_cookie(&state).await?;
    let res = put_presence(
        &state,
        &cookie,
        serde_json::json!({ "status": "away", "activity": null }),
    )
    .await?;

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(database.presence().get().await?, BotPresence::default());
    Ok(())
}

#[tokio::test]
async fn presence_does_not_require_guild_data() -> Result<(), Box<dyn Error>> {
    let (state, _receiver, _database) = state_with_presence().await;
    let cookie = common::login_cookie(&state).await?;

    let res = get_presence(&state, &cookie).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let res = put_presence(
        &state,
        &cookie,
        serde_json::json!({ "status": "invisible", "activity": null }),
    )
    .await?;
    assert_eq!(res.status(), StatusCode::OK);
    Ok(())
}
