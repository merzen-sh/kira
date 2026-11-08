use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(title = "Kira API", version = env!("CARGO_PKG_VERSION"), description = "Kira service API"),
    tags((name = "health", description = "Service health")),
    paths(kira_server::health),
    components(schemas(kira_protocol::HealthResponse))
)]
pub struct ApiDoc;

pub fn yaml() -> Result<String, Box<dyn std::error::Error>> {
    Ok(ApiDoc::openapi().to_yaml()?)
}
