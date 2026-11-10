//! Build script that ensures a dashboard placeholder exists for debug builds.

use std::error::Error;
use std::path::Path;
use std::{env, fs, io};

const PLACEHOLDER: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Kira Dashboard</title>
  </head>
  <body>
    <div id="root"></div>
    <p>Dashboard not built. Run <code>make dashboard</code>.</p>
  </body>
</html>
"#;

fn main() -> Result<(), Box<dyn Error>> {
    let manifest = env::var("CARGO_MANIFEST_DIR")?;
    let dist = Path::new(&manifest).join("../../apps/dashboard/dist");
    println!("cargo:rerun-if-changed={}", dist.display());

    let index = dist.join("index.html");
    let marker = dist.join(".kira-placeholder");
    let release = env::var("PROFILE").as_deref() == Ok("release");

    if release && (!index.exists() || marker.exists()) {
        return Err(io::Error::other(
            "dashboard assets missing; run `make dashboard` before a release build",
        )
        .into());
    }

    if !index.exists() {
        fs::create_dir_all(&dist)?;
        fs::write(&index, PLACEHOLDER)?;
        fs::write(&marker, "")?;
    }

    Ok(())
}
