use std::process::Command;

#[test]
fn version_flag_prints_name_and_version() {
    let out = Command::new(env!("CARGO_BIN_EXE_kira"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.starts_with("kira "));
}

#[test]
fn help_flag_exits_successfully() {
    let out = Command::new(env!("CARGO_BIN_EXE_kira"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
}
