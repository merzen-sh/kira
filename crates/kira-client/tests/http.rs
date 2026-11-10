use std::error::Error;

use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use kira_protocol::HealthResponse;
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
    let app = Router::new().route("/health", get(|| async { Json(HealthResponse::ok()) }));
    let server = spawn(app).await?;
    Ok((kira_client::Client::new(server.base_url.clone()), server))
}

async fn spawn_status(
    status: StatusCode,
) -> Result<(kira_client::Client, TestServer), Box<dyn Error>> {
    let app = Router::new().route("/health", get(move || async move { status }));
    let server = spawn(app).await?;
    Ok((kira_client::Client::new(server.base_url.clone()), server))
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
