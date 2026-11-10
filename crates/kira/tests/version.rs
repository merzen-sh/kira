//! Command-line flag integration tests.

use std::error::Error;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

fn temp_cwd() -> Result<PathBuf, Box<dyn Error>> {
    let unique = format!(
        "kira-version-test-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let dir = env::temp_dir().join(unique);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn run_flag(flag: &str) -> Result<Output, Box<dyn Error>> {
    let cwd = temp_cwd()?;
    let out = Command::new(env!("CARGO_BIN_EXE_kira"))
        .arg(flag)
        .current_dir(&cwd)
        .env_remove("DISCORD_TOKEN")
        .env_remove("KIRA_BIND")
        .output()?;
    fs::remove_dir_all(&cwd)?;
    Ok(out)
}

#[test]
fn version_flag_prints_name_and_version() -> Result<(), Box<dyn Error>> {
    let out = run_flag("--version")?;
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout)?;
    assert_eq!(stdout.trim(), format!("kira {}", env!("CARGO_PKG_VERSION")));
    Ok(())
}

#[test]
fn help_flag_exits_successfully() -> Result<(), Box<dyn Error>> {
    let out = run_flag("--help")?;
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout)?;
    assert!(stdout.contains("Usage: kira"));
    Ok(())
}
