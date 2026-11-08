use std::{env, fs, path::Path};

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

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dist = Path::new(&manifest).join("../../apps/dashboard/dist");
    println!("cargo:rerun-if-changed={}", dist.display());

    let index = dist.join("index.html");
    let marker = dist.join(".kira-placeholder");
    let release = env::var("PROFILE").as_deref() == Ok("release");

    if release && (!index.exists() || marker.exists()) {
        panic!("dashboard assets missing; run `make dashboard` before a release build");
    }

    if !index.exists() {
        fs::create_dir_all(&dist).unwrap();
        fs::write(&index, PLACEHOLDER).unwrap();
        fs::write(&marker, "").unwrap();
    }
}
