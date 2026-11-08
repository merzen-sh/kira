use std::fs;

#[test]
fn generate_writes_the_health_path() {
    let dir = tempfile::tempdir().unwrap();
    xtask::openapi::generate(dir.path()).unwrap();
    let yaml = fs::read_to_string(dir.path().join("openapi/openapi.yaml")).unwrap();
    assert!(yaml.contains("openapi: 3.1.0"));
    assert!(yaml.contains("/health:"));
    assert!(yaml.ends_with('\n'));
}

#[test]
fn check_accepts_fresh_output() {
    let dir = tempfile::tempdir().unwrap();
    xtask::openapi::generate(dir.path()).unwrap();
    assert!(xtask::openapi::check(dir.path()).is_ok());
}

#[test]
fn check_rejects_drift() {
    let dir = tempfile::tempdir().unwrap();
    xtask::openapi::generate(dir.path()).unwrap();
    let path = dir.path().join("openapi/openapi.yaml");
    let mut yaml = fs::read_to_string(&path).unwrap();
    yaml.push_str("# drift\n");
    fs::write(&path, yaml).unwrap();
    assert!(xtask::openapi::check(dir.path()).is_err());
}

#[test]
fn check_rejects_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    assert!(xtask::openapi::check(dir.path()).is_err());
}
