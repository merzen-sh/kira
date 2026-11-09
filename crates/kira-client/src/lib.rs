//! HTTP and WebSocket client for a remote Kira server.

use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;

pub use tokio_tungstenite::tungstenite::Message;
pub use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// Errors produced by the remote client.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("websocket error: {0}")]
    Ws(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("invalid url: {0}")]
    InvalidUrl(String),
}

/// A client for a remote Kira server.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    pub async fn health(&self) -> Result<kira_protocol::HealthResponse, ClientError> {
        let url = format!("{}/health", self.base_url.trim_end_matches('/'));
        let response = self.http.get(url).send().await?.error_for_status()?;
        Ok(response.json().await?)
    }

    pub async fn connect_ws(
        &self,
        path: &str,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, ClientError> {
        let url = ws_url(&self.base_url, path)?;
        let (stream, _response) = connect_async(url).await?;
        Ok(stream)
    }
}

fn ws_url(base: &str, path: &str) -> Result<String, ClientError> {
    let mut url =
        reqwest::Url::parse(base).map_err(|error| ClientError::InvalidUrl(error.to_string()))?;

    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        other => {
            return Err(ClientError::InvalidUrl(format!(
                "unsupported scheme `{other}`: expected http or https"
            )));
        }
    };
    url.set_scheme(scheme).map_err(|()| {
        ClientError::InvalidUrl(format!("cannot apply the `{scheme}` scheme to `{base}`"))
    })?;

    let path = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    url.set_path(&path);

    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_url_maps_http_to_ws() -> Result<(), ClientError> {
        assert_eq!(ws_url("http://x:1", "/ws")?, "ws://x:1/ws");
        Ok(())
    }

    #[test]
    fn ws_url_maps_https_to_wss() -> Result<(), ClientError> {
        assert_eq!(ws_url("https://x", "/ws")?, "wss://x/ws");
        Ok(())
    }

    #[test]
    fn ws_url_adds_a_leading_slash() -> Result<(), ClientError> {
        assert_eq!(ws_url("http://x:1/", "ws")?, "ws://x:1/ws");
        Ok(())
    }

    #[test]
    fn ws_url_rejects_other_schemes() {
        assert!(matches!(
            ws_url("ftp://x", "/ws"),
            Err(ClientError::InvalidUrl(_))
        ));
    }

    #[test]
    fn ws_url_rejects_unparseable_urls() {
        assert!(matches!(
            ws_url("not a url", "/ws"),
            Err(ClientError::InvalidUrl(_))
        ));
    }
}
