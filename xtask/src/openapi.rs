use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

const OPENAPI_PATH: &str = "openapi/openapi.yaml";

fn output_path(root: &Path) -> PathBuf {
    root.join(OPENAPI_PATH)
}

fn document() -> Result<String, Box<dyn Error>> {
    let yaml = kira_api_docs::yaml()?;
    let mut output = yaml.trim_end_matches('\n').to_owned();
    output.push('\n');
    Ok(output)
}

pub fn generate(root: &Path) -> Result<(), Box<dyn Error>> {
    let path = output_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, document()?)?;
    Ok(())
}

pub fn check(root: &Path) -> Result<(), Box<dyn Error>> {
    let path = output_path(root);
    let expected = document()?;
    let actual = fs::read(&path).map_err(|error| {
        format!(
            "{}: cannot read committed OpenAPI document ({error}); run `cargo xtask openapi generate`",
            path.display()
        )
    })?;
    if actual != expected.as_bytes() {
        return Err(format!(
            "{}: committed OpenAPI document is out of date; run `cargo xtask openapi generate`",
            path.display()
        )
        .into());
    }
    Ok(())
}
