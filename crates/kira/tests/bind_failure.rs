//! Regression tests for startup failure handling.

use std::error::Error;
use std::io;
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::time::Duration;

// Regression test for the shutdown hang when the server fails to bind before
// the bot has instantiated any shards. `ShardManager::shutdown_all()` is a
// no-op with no shard runners, so the bot must not await its gateway future.
#[tokio::test]
async fn server_bind_failure_exits_instead_of_hanging() -> Result<(), Box<dyn Error>> {
    // Hold the port so the composed binary cannot bind it.
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let bind = listener.local_addr()?;

    let mut child = Command::new(env!("CARGO_BIN_EXE_kira"))
        .env("DISCORD_TOKEN", "not-a-real-token")
        .env("KIRA_BIND", bind.to_string())
        .env("KIRA_GUILD_ID", "1")
        // Fail the gateway lookup quickly instead of reaching the network.
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .env("https_proxy", "http://127.0.0.1:1")
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("http_proxy", "http://127.0.0.1:1")
        .env_remove("ALL_PROXY")
        .env_remove("all_proxy")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let status = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok::<_, io::Error>(status);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;

    match status {
        Ok(status) => {
            let status = status?;
            assert!(
                !status.success(),
                "server bind failure should exit with an error"
            );
        }
        Err(_) => {
            child.kill()?;
            child.wait()?;
            return Err(io::Error::other("kira hung after the server failed to bind").into());
        }
    }

    drop(listener);
    Ok(())
}
