//! Workspace task runner for OpenAPI generation and checks.

use std::path::Path;
use std::process::ExitCode;

use anyhow::Context;

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> anyhow::Result<()> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .context("xtask manifest directory has a parent")?;

    match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [command, action] if command == "openapi" => match action.as_str() {
            "generate" => xtask::openapi::generate(root),
            "check" => xtask::openapi::check(root),
            _ => usage(),
        },
        _ => usage(),
    }
}

fn usage() -> ! {
    eprintln!("usage: cargo xtask openapi <generate|check>");
    std::process::exit(2);
}
