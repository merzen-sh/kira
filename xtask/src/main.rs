use std::{path::Path, process::ExitCode};

fn main() -> ExitCode {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .expect("xtask manifest directory has a parent");
    match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [command, action] if command == "openapi" => match action.as_str() {
            "generate" => run(xtask::openapi::generate(root)),
            "check" => run(xtask::openapi::check(root)),
            _ => usage(),
        },
        _ => usage(),
    }
}

fn run(result: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: cargo xtask openapi <generate|check>");
    ExitCode::from(2)
}
