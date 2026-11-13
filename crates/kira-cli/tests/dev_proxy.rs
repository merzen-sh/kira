//! Vite dev proxy integration tests.

use anyhow::Result;
use axum::body::Body;
use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing};
use http_body_util::BodyExt;
use kira_cli::{AppState, router};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tower::ServiceExt;

struct TestServer {
    base_url: String,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn(app: Router) -> Result<TestServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let base_url = format!("http://{}", listener.local_addr()?);
    let task = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            eprintln!("test server failed: {error}");
        }
    });

    Ok(TestServer { base_url, task })
}

async fn spawn_vite() -> Result<TestServer> {
    let app = Router::new()
        .route(
            "/assets/app.js",
            routing::get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    "console.log('vite')",
                )
            }),
        )
        .fallback(routing::get(vite_fallback));
    spawn(app).await
}

async fn vite_fallback(request: Request) -> Response {
    let path = request
        .uri()
        .path_and_query()
        .map(|path| path.as_str())
        .unwrap_or("/");
    (
        [(header::CONTENT_TYPE, "text/html")],
        format!("vite:{path}"),
    )
        .into_response()
}

fn app(dev_server_url: Option<String>) -> Router {
    router(AppState::new(dev_server_url))
}

async fn request(app: Router, path: &str) -> Result<Response> {
    Ok(app
        .oneshot(Request::builder().uri(path).body(Body::empty())?)
        .await?)
}

async fn body_string(res: Response) -> Result<String> {
    let bytes = res.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn dev_mode_proxies_spa_requests_to_vite() -> Result<()> {
    let vite = spawn_vite().await?;
    let res = request(
        app(Some(vite.base_url.clone())),
        "/dashboard/settings?tab=1",
    )
    .await?;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "text/html");
    assert_eq!(body_string(res).await?, "vite:/dashboard/settings?tab=1");
    Ok(())
}

#[tokio::test]
async fn dev_mode_proxies_assets_to_vite() -> Result<()> {
    let vite = spawn_vite().await?;
    let res = request(app(Some(vite.base_url.clone())), "/assets/app.js").await?;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "text/javascript");
    assert_eq!(body_string(res).await?, "console.log('vite')");
    Ok(())
}

#[tokio::test]
async fn dev_mode_does_not_proxy_unknown_api_paths() -> Result<()> {
    let vite = spawn_vite().await?;
    let res = request(app(Some(vite.base_url.clone())), "/api/does-not-exist").await?;

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert!(!body_string(res).await?.contains("vite:"));
    Ok(())
}

#[tokio::test]
async fn dev_mode_reports_unreachable_vite_as_502() -> Result<()> {
    let res = request(app(Some("http://127.0.0.1:1".into())), "/").await?;

    assert_eq!(res.status(), StatusCode::BAD_GATEWAY);
    Ok(())
}
