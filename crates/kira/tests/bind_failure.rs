use std::{
    net::TcpListener,
    process::{Command, Stdio},
    time::Duration,
};

// Regression test for the shutdown hang when the server fails to bind before
// the bot has instantiated any shards. `ShardManager::shutdown_all()` is a
// no-op with no shard runners, so the bot must not await its gateway future.
#[tokio::test]
async fn server_bind_failure_exits_instead_of_hanging() {
    // Hold the port so the composed binary cannot bind it.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let bind = listener.local_addr().unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_kira"))
        .env("DISCORD_TOKEN", "not-a-real-token")
        .env("KIRA_BIND", bind.to_string())
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
        .spawn()
        .unwrap();

    let status = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;

    match status {
        Ok(status) => assert!(
            !status.success(),
            "server bind failure should exit with an error"
        ),
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kira hung after the server failed to bind");
        }
    }

    drop(listener);
}
