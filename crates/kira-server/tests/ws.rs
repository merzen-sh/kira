use futures_util::{SinkExt, StreamExt};
use kira_server::serve_with_listener;
use tokio::sync::oneshot;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::test]
async fn ws_echoes_text_frames() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        serve_with_listener(listener, async {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let (mut socket, _response) = connect_async(format!("ws://{addr}/ws")).await.unwrap();
    socket.send(Message::Text("hello".into())).await.unwrap();
    let echoed = socket.next().await.unwrap().unwrap();
    assert_eq!(echoed, Message::Text("hello".into()));

    socket.close(None).await.unwrap();
    let _ = shutdown_tx.send(());
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn ws_echoes_binary_frames() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        serve_with_listener(listener, async {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let (mut socket, _response) = connect_async(format!("ws://{addr}/ws")).await.unwrap();
    let payload = vec![0x01, 0x02, 0x03];
    socket
        .send(Message::Binary(payload.clone().into()))
        .await
        .unwrap();
    let echoed = socket.next().await.unwrap().unwrap();
    assert_eq!(echoed, Message::Binary(payload.into()));

    socket.close(None).await.unwrap();
    let _ = shutdown_tx.send(());
    server.await.unwrap().unwrap();
}
