use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_cwd() -> PathBuf {
    let unique = format!(
        "kira-version-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let dir = env::temp_dir().join(unique);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_flag(flag: &str) -> std::process::Output {
    let cwd = temp_cwd();
    let out = Command::new(env!("CARGO_BIN_EXE_kira"))
        .arg(flag)
        .current_dir(&cwd)
        .env_remove("DISCORD_TOKEN")
        .env_remove("KIRA_BIND")
        .output()
        .unwrap();
    fs::remove_dir_all(&cwd).unwrap();
    out
}

#[test]
fn version_flag_prints_name_and_version() {
    let out = run_flag("--version");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.trim(), format!("kira {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_flag_exits_successfully() {
    let out = run_flag("--help");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Usage: kira"));
}
