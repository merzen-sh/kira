//! API token authentication tests.

use std::error::Error;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use kira_server::{AppState, router};
use tokio::sync::{mpsc, watch};
use tower::ServiceExt;

fn state(api_token: Option<&str>) -> AppState {
    let (_guild_sender, guilds) = watch::channel(Vec::new());
    let (_channel_sender, channels) = watch::channel(Vec::new());
    let (messages, _receiver) = mpsc::channel(1);

    AppState::new(1, guilds, channels, messages, api_token.map(str::to_owned))
}

async fn get_guild(
    state: AppState,
    authorization: Option<&str>,
) -> Result<StatusCode, Box<dyn Error>> {
    let mut request = Request::builder().uri("/guild");

    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }

    let response = router(state).oneshot(request.body(Body::empty())?).await?;

    Ok(response.status())
}

#[tokio::test]
async fn requests_without_a_token_are_rejected() -> Result<(), Box<dyn Error>> {
    let status = get_guild(state(Some("secret")), None).await?;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn a_wrong_token_is_rejected() -> Result<(), Box<dyn Error>> {
    let status = get_guild(state(Some("secret")), Some("Bearer other")).await?;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn the_configured_token_is_accepted() -> Result<(), Box<dyn Error>> {
    let status = get_guild(state(Some("secret")), Some("Bearer secret")).await?;

    // The token passed; the guild cache is simply empty.
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    Ok(())
}

#[tokio::test]
async fn health_stays_public() -> Result<(), Box<dyn Error>> {
    let response = router(state(Some("secret")))
        .oneshot(Request::builder().uri("/health").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}
