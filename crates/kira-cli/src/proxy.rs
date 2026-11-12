//! HTTP and WebSocket proxies to the remote Kira server.

use std::future::Future;

use axum::Json;
use axum::extract::ws::{
    CloseFrame as AxumCloseFrame, Message as AxumMessage, WebSocket, WebSocketUpgrade,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use futures_util::{SinkExt, StreamExt};
use kira_client::{Client, ClientError};
use kira_protocol::ErrorResponse;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as TsMessage;
use tokio_tungstenite::tungstenite::protocol::CloseFrame as TsCloseFrame;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::AppState;

/// Proxy the remote health endpoint.
pub(crate) async fn health(State(state): State<AppState>) -> Response {
    match state.client.health().await {
        Ok(health) => Json(health).into_response(),
        Err(error) => {
            eprintln!("health proxy failed: {error}");
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new("upstream service unavailable")),
            )
                .into_response()
        }
    }
}

/// Proxy the remote guild endpoint.
pub(crate) async fn guild(State(state): State<AppState>) -> Response {
    optional_proxy("guild", async move {
        state.client.guild().await.map(|guild| guild.map(Json))
    })
    .await
}

/// Proxy the remote guild channels endpoint.
pub(crate) async fn channels(State(state): State<AppState>) -> Response {
    optional_proxy("channels", async move {
        state
            .client
            .channels()
            .await
            .map(|channels| channels.map(Json))
    })
    .await
}

/// Proxy a message posted to a guild channel.
pub(crate) async fn send_message(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    match state.client.send_message(&channel_id, &payload).await {
        Ok(message) => (StatusCode::CREATED, Json(message)).into_response(),
        Err(error) => {
            eprintln!("message proxy failed: {error}");
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new("upstream service unavailable")),
            )
                .into_response()
        }
    }
}

/// Proxy an endpoint that returns `503` when its data is not available.
async fn optional_proxy<T, F>(label: &str, fetch: F) -> Response
where
    T: IntoResponse,
    F: Future<Output = Result<Option<T>, ClientError>>,
{
    match fetch.await {
        Ok(Some(value)) => value.into_response(),
        Ok(None) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new("guild data is not available")),
        )
            .into_response(),
        Err(error) => {
            eprintln!("{label} proxy failed: {error}");
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new("upstream service unavailable")),
            )
                .into_response()
        }
    }
}

/// Upgrade the local connection and bridge it to the remote `/ws` endpoint.
pub(crate) async fn ws_upgrade(
    State(state): State<AppState>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade.on_upgrade(move |socket| proxy_socket(socket, state.client))
}

async fn proxy_socket(socket: WebSocket, client: Client) {
    match client.connect_ws("/ws").await {
        Ok(remote) => bridge(socket, remote).await,
        Err(error) => {
            eprintln!("websocket proxy failed to reach the remote server: {error}");
            let mut socket = socket;
            if let Err(send_error) = socket
                .send(AxumMessage::Close(Some(AxumCloseFrame {
                    code: 1011,
                    reason: "remote websocket unavailable".into(),
                })))
                .await
            {
                eprintln!("failed to send websocket close frame: {send_error}");
            }
        }
    }
}

async fn bridge(socket: WebSocket, remote: WebSocketStream<MaybeTlsStream<TcpStream>>) {
    let (mut client_tx, mut client_rx) = socket.split();
    let (mut remote_tx, mut remote_rx) = remote.split();

    let client_to_remote = async {
        while let Some(result) = client_rx.next().await {
            let message = match result {
                Ok(message) => message,
                Err(error) => {
                    eprintln!("client websocket receive failed: {error}");
                    break;
                }
            };
            let closing = matches!(message, AxumMessage::Close(_));
            if let Err(error) = remote_tx.send(to_tungstenite(message)).await {
                eprintln!("forwarding to the remote websocket failed: {error}");
                break;
            }
            if closing {
                break;
            }
        }
    };

    let remote_to_client = async {
        while let Some(result) = remote_rx.next().await {
            let message = match result {
                Ok(message) => message,
                Err(error) => {
                    eprintln!("remote websocket receive failed: {error}");
                    break;
                }
            };
            let closing = matches!(message, TsMessage::Close(_));
            if let Err(error) = client_tx.send(to_axum(message)).await {
                eprintln!("forwarding to the local websocket failed: {error}");
                break;
            }
            if closing {
                break;
            }
        }
    };

    tokio::select! {
        () = client_to_remote => {}
        () = remote_to_client => {}
    }
}

fn to_tungstenite(message: AxumMessage) -> TsMessage {
    match message {
        AxumMessage::Text(text) => TsMessage::Text(text.as_str().into()),
        AxumMessage::Binary(data) => TsMessage::Binary(data.to_vec().into()),
        AxumMessage::Ping(data) => TsMessage::Ping(data.to_vec().into()),
        AxumMessage::Pong(data) => TsMessage::Pong(data.to_vec().into()),
        AxumMessage::Close(frame) => TsMessage::Close(frame.map(|frame| TsCloseFrame {
            code: frame.code.into(),
            reason: frame.reason.as_str().into(),
        })),
    }
}

fn to_axum(message: TsMessage) -> AxumMessage {
    match message {
        TsMessage::Text(text) => AxumMessage::Text(text.as_str().into()),
        TsMessage::Binary(data) => AxumMessage::Binary(data.to_vec().into()),
        TsMessage::Ping(data) => AxumMessage::Ping(data.to_vec().into()),
        TsMessage::Pong(data) => AxumMessage::Pong(data.to_vec().into()),
        TsMessage::Close(frame) => AxumMessage::Close(frame.map(|frame| AxumCloseFrame {
            code: frame.code.into(),
            reason: frame.reason.as_str().into(),
        })),
        // Tungstenite never yields raw frames while reading, but preserve any
        // payload rather than dropping it.
        TsMessage::Frame(frame) => AxumMessage::Binary(frame.into_payload().to_vec().into()),
    }
}

#[cfg(test)]
mod tests {
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

    use super::*;

    #[test]
    fn converts_text_frames() {
        let axum = AxumMessage::Text("hello".into());
        let ts = to_tungstenite(axum.clone());
        assert_eq!(ts, TsMessage::Text("hello".into()));
        assert_eq!(to_axum(ts), axum);
    }

    #[test]
    fn converts_binary_frames() {
        let axum = AxumMessage::Binary(vec![0x01, 0x02, 0x03].into());
        let ts = to_tungstenite(axum.clone());
        assert_eq!(ts, TsMessage::Binary(vec![0x01, 0x02, 0x03].into()));
        assert_eq!(to_axum(ts), axum);
    }

    #[test]
    fn converts_ping_frames() {
        let axum = AxumMessage::Ping(vec![0xaa].into());
        let ts = to_tungstenite(axum.clone());
        assert_eq!(ts, TsMessage::Ping(vec![0xaa].into()));
        assert_eq!(to_axum(ts), axum);
    }

    #[test]
    fn converts_pong_frames() {
        let axum = AxumMessage::Pong(vec![0xbb].into());
        let ts = to_tungstenite(axum.clone());
        assert_eq!(ts, TsMessage::Pong(vec![0xbb].into()));
        assert_eq!(to_axum(ts), axum);
    }

    #[test]
    fn converts_close_frames() {
        let axum = AxumMessage::Close(Some(AxumCloseFrame {
            code: 1000,
            reason: "bye".into(),
        }));
        let ts = to_tungstenite(axum.clone());
        assert_eq!(
            ts,
            TsMessage::Close(Some(TsCloseFrame {
                code: CloseCode::Normal,
                reason: "bye".into(),
            }))
        );
        assert_eq!(to_axum(ts), axum);
    }

    #[test]
    fn converts_empty_close_frames() {
        let axum = AxumMessage::Close(None);
        let ts = to_tungstenite(axum.clone());
        assert_eq!(ts, TsMessage::Close(None));
        assert_eq!(to_axum(ts), axum);
    }
}
