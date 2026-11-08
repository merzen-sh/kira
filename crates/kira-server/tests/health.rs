use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use kira_protocol::HealthResponse;
use kira_server::router;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let res = router()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let health: HealthResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(health.status, "ok");
}

#[tokio::test]
async fn unknown_path_is_404() {
    let res = router()
        .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
