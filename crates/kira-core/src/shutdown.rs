use std::io;

#[cfg(unix)]
pub async fn wait_for_signal() -> io::Result<()> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate())?;

    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}

#[cfg(not(unix))]
pub async fn wait_for_signal() -> io::Result<()> {
    tokio::signal::ctrl_c().await
}
