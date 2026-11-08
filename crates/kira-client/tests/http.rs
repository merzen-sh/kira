use axum::{Json, Router, http::StatusCode, routing::get};
use kira_protocol::HealthResponse;
use tokio::{net::TcpListener, task::JoinHandle};

struct TestServer {
    base_url: String,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn(app: Router) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    TestServer { base_url, task }
}

async fn spawn_health() -> (kira_client::Client, TestServer) {
    let app = Router::new().route("/health", get(|| async { Json(HealthResponse::ok()) }));
    let server = spawn(app).await;
    (kira_client::Client::new(server.base_url.clone()), server)
}

async fn spawn_status(status: StatusCode) -> (kira_client::Client, TestServer) {
    let app = Router::new().route("/health", get(move || async move { status }));
    let server = spawn(app).await;
    (kira_client::Client::new(server.base_url.clone()), server)
}

#[tokio::test]
async fn health_returns_the_body() {
    let (client, _server) = spawn_health().await;
    let health = client.health().await.unwrap();
    assert_eq!(health.status, "ok");
    assert_eq!(health.version, HealthResponse::ok().version);
}

#[tokio::test]
async fn health_maps_http_errors() {
    let (client, _server) = spawn_status(StatusCode::INTERNAL_SERVER_ERROR).await;
    assert!(matches!(
        client.health().await,
        Err(kira_client::ClientError::Http(_))
    ));
}

#[tokio::test]
async fn health_tolerates_a_trailing_slash_in_the_base_url() {
    let (_client, server) = spawn_health().await;
    let client = kira_client::Client::new(format!("{}/", server.base_url));
    assert_eq!(client.health().await.unwrap().status, "ok");
}
