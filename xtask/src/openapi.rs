//! Generate and verify the committed OpenAPI document.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

const OPENAPI_PATH: &str = "openapi/openapi.yaml";

fn output_path(root: &Path) -> PathBuf {
    root.join(OPENAPI_PATH)
}

fn document() -> Result<String> {
    let yaml = kira_api_docs::yaml().context("building the OpenAPI document")?;
    let mut output = yaml.trim_end_matches('\n').to_owned();
    output.push('\n');
    Ok(output)
}

/// Write the OpenAPI document to `openapi/openapi.yaml` under `root`.
pub fn generate(root: &Path) -> Result<()> {
    let path = output_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&path, document()?).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Fail when the committed OpenAPI document differs from the generated one.
pub fn check(root: &Path) -> Result<()> {
    let path = output_path(root);
    let expected = document()?;
    let actual = fs::read(&path).with_context(|| {
        format!(
            "{}: cannot read committed OpenAPI document; run `cargo xtask openapi generate`",
            path.display()
        )
    })?;
    if actual != expected.as_bytes() {
        bail!(
            "{}: committed OpenAPI document is out of date; run `cargo xtask openapi generate`",
            path.display()
        );
    }
    Ok(())
}
