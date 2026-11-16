//! Wait for termination signals from the operating system.

use std::io;

/// Wait for Ctrl+C or, on Unix, a `SIGTERM` signal.
///
/// # Errors
///
/// Returns an error when the signal handler cannot be installed.
#[cfg(unix)]
pub async fn wait_for_signal() -> io::Result<()> {
    use tokio::signal::unix;

    let mut terminate = unix::signal(unix::SignalKind::terminate())?;

    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}

/// Wait for Ctrl+C.
///
/// # Errors
///
/// Returns an error when the signal handler cannot be installed.
#[cfg(not(unix))]
pub async fn wait_for_signal() -> io::Result<()> {
    tokio::signal::ctrl_c().await
}
