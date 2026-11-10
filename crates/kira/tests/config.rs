//! Configuration loading integration tests.

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::tempdir;

fn run_in(dir: &Path) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_kira"))
        .current_dir(dir)
        .env_remove("DISCORD_TOKEN")
        .env_remove("KIRA_BIND")
        .output()?)
}

#[test]
fn missing_env_file_is_tolerated() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let out = run_in(dir.path())?;
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr)?;
    assert!(stderr.contains("failed to parse environment variables"));
    assert!(!stderr.contains("failed to load .env file"));
    Ok(())
}

#[test]
fn malformed_env_file_does_not_echo_secrets() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    fs::write(
        dir.path().join(".env"),
        "DISCORD_TOKEN=super-secret\nthis is not valid\n",
    )?;

    let out = run_in(dir.path())?;
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr)?;
    assert!(stderr.contains("failed to parse .env content near index"));
    assert!(!stderr.contains("super-secret"));
    Ok(())
}
