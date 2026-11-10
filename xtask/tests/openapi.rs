//! OpenAPI task integration tests.

use std::fs;

#[test]
fn generate_writes_the_health_path() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    xtask::openapi::generate(dir.path())?;
    let yaml = fs::read_to_string(dir.path().join("openapi/openapi.yaml"))?;
    assert!(yaml.contains("openapi: 3.1.0"));
    assert!(yaml.contains("/health:"));
    assert!(yaml.ends_with('\n'));
    Ok(())
}

#[test]
fn check_accepts_fresh_output() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    xtask::openapi::generate(dir.path())?;
    assert!(xtask::openapi::check(dir.path()).is_ok());
    Ok(())
}

#[test]
fn check_rejects_drift() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    xtask::openapi::generate(dir.path())?;
    let path = dir.path().join("openapi/openapi.yaml");
    let mut yaml = fs::read_to_string(&path)?;
    yaml.push_str("# drift\n");
    fs::write(&path, yaml)?;
    assert!(xtask::openapi::check(dir.path()).is_err());
    Ok(())
}

#[test]
fn check_rejects_missing_file() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    assert!(xtask::openapi::check(dir.path()).is_err());
    Ok(())
}
