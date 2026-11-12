//! HTTP client integration tests.

use std::error::Error;

use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::{Json, Router, routing};
use kira_protocol::{ChannelDetails, ErrorResponse, GuildDetails, HealthResponse, SentMessage};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

struct TestServer {
    base_url: String,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn(app: Router) -> Result<TestServer, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let base_url = format!("http://{}", listener.local_addr()?);
    let task = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            eprintln!("test server failed: {error}");
        }
    });

    Ok(TestServer { base_url, task })
}

async fn spawn_health() -> Result<(kira_client::Client, TestServer), Box<dyn Error>> {
    let app = Router::new().route(
        "/health",
        routing::get(|| async { Json(HealthResponse::ok()) }),
    );
    let server = spawn(app).await?;
    Ok((kira_client::Client::new(server.base_url.clone()), server))
}

async fn spawn_status(
    status: StatusCode,
) -> Result<(kira_client::Client, TestServer), Box<dyn Error>> {
    let app = Router::new().route("/health", routing::get(move || async move { status }));
    let server = spawn(app).await?;
    Ok((kira_client::Client::new(server.base_url.clone()), server))
}

async fn spawn_guild() -> Result<(kira_client::Client, TestServer), Box<dyn Error>> {
    let mut guild = GuildDetails::new("1", "Alpha");
    guild.member_count = 12;

    let app = Router::new().route(
        "/guild",
        routing::get(move || {
            let guild = guild.clone();
            async move { Json(guild) }
        }),
    );
    let server = spawn(app).await?;
    Ok((kira_client::Client::new(server.base_url.clone()), server))
}

async fn spawn_guild_unavailable() -> Result<(kira_client::Client, TestServer), Box<dyn Error>> {
    let app = Router::new().route(
        "/guild",
        routing::get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
    );
    let server = spawn(app).await?;
    Ok((kira_client::Client::new(server.base_url.clone()), server))
}

async fn spawn_channels() -> Result<(kira_client::Client, TestServer), Box<dyn Error>> {
    let channels = vec![ChannelDetails::new("10", "1", "general", "text")];

    let app = Router::new().route(
        "/guild/channels",
        routing::get(move || {
            let channels = channels.clone();
            async move { Json(channels) }
        }),
    );
    let server = spawn(app).await?;
    Ok((kira_client::Client::new(server.base_url.clone()), server))
}

async fn spawn_channels_unavailable() -> Result<(kira_client::Client, TestServer), Box<dyn Error>> {
    let app = Router::new().route(
        "/guild/channels",
        routing::get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
    );
    let server = spawn(app).await?;
    Ok((kira_client::Client::new(server.base_url.clone()), server))
}

async fn spawn_send_message() -> Result<(kira_client::Client, TestServer), Box<dyn Error>> {
    let app = Router::new().route(
        "/guild/channels/{channel_id}/messages",
        routing::post(|| async { Json(SentMessage::new("99", "10")) }),
    );
    let server = spawn(app).await?;
    Ok((kira_client::Client::new(server.base_url.clone()), server))
}

#[tokio::test]
async fn requests_carry_the_api_token() -> Result<(), Box<dyn Error>> {
    let app = Router::new().route(
        "/health",
        routing::get(|headers: axum::http::HeaderMap| async move {
            let authorized = headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                == Some("Bearer secret");

            if authorized {
                (StatusCode::OK, Json(HealthResponse::ok())).into_response()
            } else {
                StatusCode::UNAUTHORIZED.into_response()
            }
        }),
    );
    let server = spawn(app).await?;
    let client =
        kira_client::Client::new(server.base_url.clone()).with_token(Some("secret".into()));

    let health = client.health().await?;

    assert_eq!(health.status, "ok");
    Ok(())
}

#[tokio::test]
async fn message_is_posted_to_the_channel() -> Result<(), Box<dyn Error>> {
    let (client, _server) = spawn_send_message().await?;

    let message = client
        .send_message("10", &serde_json::json!({ "content": "Hello" }))
        .await?;
    assert_eq!(message.id, "99");
    assert_eq!(message.channel_id, "10");
    Ok(())
}

#[tokio::test]
async fn message_error_is_propagated() -> Result<(), Box<dyn Error>> {
    let app = Router::new().route(
        "/guild/channels/{channel_id}/messages",
        routing::post(|| async {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("content or embeds is required")),
            )
        }),
    );
    let server = spawn(app).await?;
    let client = kira_client::Client::new(server.base_url.clone());

    let result = client.send_message("10", &serde_json::json!({})).await;

    match result {
        Err(kira_client::ClientError::Upstream { status, message }) => {
            assert_eq!(status, 400);
            assert_eq!(message, "content or embeds is required");
        }
        other => panic!("expected an upstream error, got {other:?}"),
    }
    Ok(())
}

#[tokio::test]
async fn channels_return_the_list() -> Result<(), Box<dyn Error>> {
    let (client, _server) = spawn_channels().await?;

    let channels = client.channels().await?.expect("channels");
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].name, "general");
    Ok(())
}

#[tokio::test]
async fn channels_map_unavailable_to_none() -> Result<(), Box<dyn Error>> {
    let (client, _server) = spawn_channels_unavailable().await?;

    assert!(client.channels().await?.is_none());
    Ok(())
}

#[tokio::test]
async fn guild_returns_the_details() -> Result<(), Box<dyn Error>> {
    let (client, _server) = spawn_guild().await?;

    let guild = client.guild().await?.expect("guild details");
    assert_eq!(guild.id, "1");
    assert_eq!(guild.name, "Alpha");
    assert_eq!(guild.member_count, 12);
    Ok(())
}

#[tokio::test]
async fn guild_maps_unavailable_to_none() -> Result<(), Box<dyn Error>> {
    let (client, _server) = spawn_guild_unavailable().await?;

    assert!(client.guild().await?.is_none());
    Ok(())
}

#[tokio::test]
async fn health_returns_the_body() -> Result<(), Box<dyn Error>> {
    let (client, _server) = spawn_health().await?;
    let health = client.health().await?;
    assert_eq!(health.status, "ok");
    assert_eq!(health.version, HealthResponse::ok().version);
    Ok(())
}

#[tokio::test]
async fn health_maps_http_errors() -> Result<(), Box<dyn Error>> {
    let (client, _server) = spawn_status(StatusCode::INTERNAL_SERVER_ERROR).await?;
    assert!(matches!(
        client.health().await,
        Err(kira_client::ClientError::Http(_))
    ));
    Ok(())
}

#[tokio::test]
async fn health_tolerates_a_trailing_slash_in_the_base_url() -> Result<(), Box<dyn Error>> {
    let (_client, server) = spawn_health().await?;
    let client = kira_client::Client::new(format!("{}/", server.base_url));
    assert_eq!(client.health().await?.status, "ok");
    Ok(())
}
