# Kira Workspace Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure Kira into a Rust workspace with independent `kira` and `kira-cli` binaries, shared libraries, a Utoipa OpenAPI pipeline, generated TypeScript types, and an embedded Vite dashboard.

**Architecture:** One Cargo workspace under `crates/` with a dev-only `xtask`. Leaf crates (`core`, `config`, `protocol`) feed library crates (`bot`, `server`, `client`), which feed the two application binaries. `kira-api-docs` is a dev-only capstone over `kira-server`; `xtask` generates and checks `openapi/openapi.yaml`. npm workspaces hold `apps/dashboard` and `packages/api-types`.

**Tech Stack:** Rust 2024 (MSRV 1.88), Axum 0.8, Utoipa 6, Serenity 0.12, rust-embed 8, reqwest 0.13, tokio-tungstenite 0.30, Vite 8, React 19, TypeScript, openapi-typescript 7.

**Spec:** `docs/superpowers/specs/2026-11-08-kira-workspace-design.md`

## Global Constraints

- Edition 2024, `resolver = "3"`, `rust-version = "1.88"`; every crate `publish = false`.
- `kira` features: `default = ["server"]`, `server = ["dep:kira-server"]`. Bot is always compiled.
- `kira-cli` must never depend on `kira-server`, `kira-bot`, Serenity, or `kira-api-docs`.
- `kira` must never depend on `kira-api-docs` or `xtask`; no application may depend on `xtask`.
- OpenAPI document assembly and YAML serialization live only in `kira-api-docs`; `utoipa` path annotations may live in `kira-server`/`kira-protocol`.
- `openapi/openapi.yaml` and `packages/api-types/src/schema.d.ts` are generated and committed.
- The YAML→TypeScript tool is `openapi-typescript` (the named `typescript-openapi` package does not exist).
- npm workspaces; `package-lock.json` is committed; install with `npm ci`.
- Never ship from `cargo build --release --workspace`; release targets name `-p kira` or `-p kira-cli`.
- `.env` is never committed; `.env.example` is.
- No business logic beyond `!ping`, `/health`, a WS echo, and transport plumbing.
- Keep existing `println!`/`eprintln!` behavior; do not add a logging crate.

## Review Focus

These are the failure modes most likely to bite despite a green unit suite. Each names the test that pins it.

1. **Missing dashboard assets in a release build.** A release `kira-cli` must fail loudly, not ship a placeholder or an empty asset set. Test: Task 7's `cargo clean -p kira-cli` + missing-`dist` build check.
2. **SPA fallback swallowing API misses.** `/api/*` routes that do not exist must return 404, never `index.html`. Test: Task 7's `api_unknown_path_is_404`.
3. **Contract drift.** A route change must fail `cargo xtask openapi check`, and a stale `schema.d.ts` must fail `npm run check:api-types`. Tests: Task 4's `check_rejects_drift`, Task 8's stale-schema check.
4. **Embedded asset path traversal.** A request containing `..` must not read outside the embedded set. Test: Task 7's `traversal_does_not_escape`.
5. **Feature/dependency leakage.** `kira-cli` must not pull in the server or bot, and release `kira` must not pull in doc tooling. Tests: Task 9's `cargo tree` assertions.

---

### Task 1: Workspace Foundation and Leaf Crates

**Files:**
- Create: `Cargo.toml` (workspace only)
- Create: `.gitignore`
- Create: `crates/kira-core/Cargo.toml`, `crates/kira-core/src/lib.rs`, `crates/kira-core/src/shutdown.rs`
- Create: `crates/kira-config/Cargo.toml`, `crates/kira-config/src/lib.rs`
- Create: `crates/kira-protocol/Cargo.toml`, `crates/kira-protocol/src/lib.rs`
- Delete: `src/main.rs`, `src/config.rs`, `src/handler.rs`, `src/shutdown.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `kira_core::BoxError = Box<dyn std::error::Error + Send + Sync>`
  - `kira_core::Result<T> = std::result::Result<T, BoxError>`
  - `kira_core::shutdown::wait_for_signal() -> std::io::Result<()>`
  - `kira_config::{DiscordConfig, ServerConfig, CliConfig, ConfigError, load_env}`
  - Each config type: `from_env() -> Result<Self, ConfigError>` and `from_iter<I, K, V>(iter: I) -> Result<Self, ConfigError>` where `I: IntoIterator<Item = (K, V)>`, `K: AsRef<str>`, `V: Into<String>`.
  - `kira_protocol::{HealthResponse, ErrorResponse}` with `HealthResponse::ok()`.

- [ ] **Step 1: Create the workspace manifest and ignore rules**

Root `Cargo.toml` is workspace-only. Start with `members = ["crates/*"]`; `xtask` is added in Task 4.

```toml
[workspace]
members = ["crates/*"]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.88"
publish = false

[workspace.dependencies]
axum = { version = "0.8", features = ["ws"] }
dotenvy = "0.15"
envy = "0.4"
futures-util = "0.3"
http-body-util = "0.1"
mime_guess = "2"
reqwest = { version = "0.13", features = ["json"] }
rust-embed = "8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serenity = "0.12"
tempfile = "3"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal", "net", "sync", "time"] }
tokio-tungstenite = { version = "0.30", features = ["rustls-tls-webpki-roots"] }
tower = { version = "0.5", features = ["util"] }
utoipa = "6"

[profile.release]
opt-level = 3
codegen-units = 1
lto = "fat"
panic = "abort"
strip = true

[profile.dev]
debug = "line-tables-only"
split-debuginfo = "unpacked"
```

`.gitignore`:

```gitignore
/target
.env
.env.*
!.env.example
node_modules/
apps/dashboard/dist/
```

- [ ] **Step 2: Write the failing config and protocol tests**

In `crates/kira-config/src/lib.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_config_reads_token() {
        let c = DiscordConfig::from_iter([("DISCORD_TOKEN", "abc")]).unwrap();
        assert_eq!(c.token, "abc");
    }

    #[test]
    fn missing_token_is_an_error() {
        assert!(DiscordConfig::from_iter(std::iter::empty::<(&str, &str)>()).is_err());
    }

    #[test]
    fn server_config_defaults_bind() {
        let c = ServerConfig::from_iter(std::iter::empty::<(&str, &str)>()).unwrap();
        assert_eq!(c.bind, "127.0.0.1:3000".parse().unwrap());
    }

    #[test]
    fn server_config_reads_kira_bind() {
        let c = ServerConfig::from_iter([("KIRA_BIND", "0.0.0.0:8080")]).unwrap();
        assert_eq!(c.bind, "0.0.0.0:8080".parse().unwrap());
    }

    #[test]
    fn cli_config_defaults() {
        let c = CliConfig::from_iter(std::iter::empty::<(&str, &str)>()).unwrap();
        assert_eq!(c.bind, "127.0.0.1:3001".parse().unwrap());
        assert_eq!(c.remote_url, "http://127.0.0.1:3000");
    }

    #[test]
    fn cli_config_reads_env() {
        let c = CliConfig::from_iter([
            ("KIRA_CLI_BIND", "127.0.0.1:4001"),
            ("KIRA_REMOTE_URL", "http://example.test:9000"),
        ])
        .unwrap();
        assert_eq!(c.bind, "127.0.0.1:4001".parse().unwrap());
        assert_eq!(c.remote_url, "http://example.test:9000");
    }
}
```

In `crates/kira-protocol/src/lib.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_ok_serializes_expected_fields() {
        let v = serde_json::to_value(HealthResponse::ok()).unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn error_response_serializes() {
        let v = serde_json::to_value(ErrorResponse { error: "boom".into() }).unwrap();
        assert_eq!(v["error"], "boom");
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p kira-config -p kira-protocol`
Expected: FAIL — the packages or types do not exist yet.

- [ ] **Step 4: Implement the three leaf crates**

`crates/kira-core/Cargo.toml` depends on `tokio.workspace = true`. `src/lib.rs` declares `pub mod shutdown;` and the error aliases. Move the current `src/shutdown.rs` into `crates/kira-core/src/shutdown.rs` unchanged.

`crates/kira-config/Cargo.toml`:

```toml
[package]
name = "kira-config"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
dotenvy.workspace = true
envy.workspace = true
serde.workspace = true
thiserror.workspace = true
```

`crates/kira-config/src/lib.rs` uses `envy::prefixed` and serde defaults. The critical details:

```rust
pub fn load_env() {
    let _ = dotenvy::dotenv();
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: SocketAddr,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CliConfig {
    #[serde(default = "default_cli_bind", rename = "cli_bind")]
    pub bind: SocketAddr,
    #[serde(default = "default_remote_url")]
    pub remote_url: String,
}
```

- `DiscordConfig::from_iter` → `envy::prefixed("DISCORD_").from_iter(iter)`.
- `ServerConfig::from_iter` → `envy::prefixed("KIRA_").from_iter(iter)`.
- `CliConfig::from_iter` → `envy::prefixed("KIRA_").from_iter(iter)`.
- Defaults: bind `127.0.0.1:3000`, CLI bind `127.0.0.1:3001`, remote URL `http://127.0.0.1:3000`.
- `ConfigError` has one variant: `#[error("Failed to parse environment variables: {0}")] Environment(#[from] envy::Error)`.
- `from_env()` calls `load_env()` then `Self::from_iter(std::env::vars())`.

`crates/kira-protocol/Cargo.toml` depends on `serde.workspace = true` and `utoipa.workspace = true`; dev-dep `serde_json.workspace = true`.

`crates/kira-protocol/src/lib.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

impl HealthResponse {
    pub fn ok() -> Self {
        Self { status: "ok".into(), version: env!("CARGO_PKG_VERSION").into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p kira-config -p kira-protocol`
Expected: PASS for all eight tests.

- [ ] **Step 6: Verify the workspace builds without the old package**

Run: `cargo build --workspace`
Expected: success. There is temporarily no application binary; Task 3 restores it.

- [ ] **Step 7: Commit (initial commit; the repository has no HEAD yet)**

```bash
git add -A
git commit -m "chore: establish cargo workspace and shared crates"
```

---

### Task 2: Axum Server Library

**Files:**
- Create: `crates/kira-server/Cargo.toml`, `crates/kira-server/src/lib.rs`
- Create: `crates/kira-server/tests/health.rs`, `crates/kira-server/tests/ws.rs`

**Interfaces:**
- Consumes: `kira_core::Result`, `kira_config::ServerConfig`, `kira_protocol::HealthResponse`.
- Produces:
  - `kira_server::router() -> axum::Router`
  - `kira_server::health() -> axum::Json<HealthResponse>` (annotated with `#[utoipa::path]`)
  - `kira_server::serve(config: &ServerConfig, shutdown: impl Future<Output = ()> + Send + 'static) -> kira_core::Result<()>`
  - `kira_server::serve_with_listener(listener: tokio::net::TcpListener, shutdown: impl Future<Output = ()> + Send + 'static) -> kira_core::Result<()>`

- [ ] **Step 1: Write the failing health test**

`crates/kira-server/tests/health.rs`:

```rust
use axum::{body::Body, http::{Request, StatusCode}};
use http_body_util::BodyExt;
use kira_protocol::HealthResponse;
use kira_server::router;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let res = router()
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let health: HealthResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(health.status, "ok");
}

#[tokio::test]
async fn unknown_path_is_404() {
    let res = router()
        .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p kira-server --test health`
Expected: FAIL — package `kira-server` does not exist.

- [ ] **Step 3: Implement the server crate**

`Cargo.toml` dependencies: `axum.workspace = true`, `kira-core`, `kira-config`, `kira-protocol`, `tokio.workspace = true`, `utoipa.workspace = true`. Dev-dependencies: `futures-util`, `http-body-util`, `serde_json`, `tokio`, `tokio-tungstenite`, `tower`.

Put the documented handler at the crate root so Utoipa's generated `__path_health` is at `kira_server::health`:

```rust
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses((status = 200, body = HealthResponse))
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}
```

`router()` mounts `/health` and `/ws` (see Step 5) and nothing else, so unknown paths are 404.

`serve` binds `config.bind`, prints the bound address, and delegates to `serve_with_listener`, which runs `axum::serve(...).with_graceful_shutdown(shutdown)` and maps errors through `kira_core::Result`.

- [ ] **Step 4: Run the health tests to verify they pass**

Run: `cargo test -p kira-server --test health`
Expected: PASS (2 tests).

- [ ] **Step 5: Write the failing WebSocket echo test**

`crates/kira-server/tests/ws.rs` starts `serve_with_listener` on `127.0.0.1:0`, reads `listener.local_addr()`, connects with `tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))`, sends `Message::Text("hello".into())`, asserts the next frame is `Message::Text` equal to `"hello"`, closes, and stops the server through a `oneshot` shutdown channel.

Run: `cargo test -p kira-server --test ws`
Expected: FAIL — the route does not exist.

- [ ] **Step 6: Implement the WS scaffold and rerun**

`/ws` uses `WebSocketUpgrade::on_upgrade`. The socket loop receives frames and sends back `Text` and `Binary` frames unchanged; it stops on `Close` or on a send/receive error. It does not interpret payloads. Add no OpenAPI annotation.

Run: `cargo test -p kira-server --test ws`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add Cargo.lock crates/kira-server
git commit -m "feat: add axum server library with health and ws scaffold"
```

---

### Task 3: Bot Library and the `kira` Binary

**Files:**
- Create: `crates/kira-bot/Cargo.toml`, `crates/kira-bot/src/lib.rs`, `crates/kira-bot/src/handler.rs`
- Create: `crates/kira/Cargo.toml`, `crates/kira/src/main.rs`, `crates/kira/src/app.rs`
- Create: `crates/kira/tests/version.rs`

**Interfaces:**
- Consumes: `kira_config::{DiscordConfig, ServerConfig}`, `kira_core::{shutdown, Result}`, `kira_server::serve`.
- Produces:
  - `kira_bot::is_ping(content: &str, author_is_bot: bool) -> bool`
  - `kira_bot::run(config: DiscordConfig, shutdown: impl Future<Output = ()> + Send) -> kira_core::Result<()>`
  - `kira_bot::{ConfigKey, Handler}`
  - Binary `kira`, honoring `--version` and `--help`.

- [ ] **Step 1: Write the failing bot unit test**

In `crates/kira-bot/src/handler.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::is_ping;

    #[test]
    fn ping_from_human_matches() {
        assert!(is_ping("!ping", false));
    }

    #[test]
    fn ping_from_bot_is_ignored() {
        assert!(!is_ping("!ping", true));
    }

    #[test]
    fn other_text_is_ignored() {
        assert!(!is_ping("hello", false));
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p kira-bot`
Expected: FAIL — package does not exist.

- [ ] **Step 3: Implement the bot crate**

Move the current `src/handler.rs` behavior into `crates/kira-bot`. `ConfigKey` stores `Arc<DiscordConfig>`. `Handler::message` uses `is_ping(msg.content.as_str(), msg.author.bot)` and keeps the existing "Pong!" reply and config lookup.

`run` takes ownership of `DiscordConfig`, wraps it in `Arc`, builds the Serenity client with `GUILD_MESSAGES | DIRECT_MESSAGES | MESSAGE_CONTENT`, inserts `ConfigKey`, pins `client.start()`, and uses `tokio::select!` between `start` and the supplied shutdown future. On shutdown it calls `shard_manager.shutdown_all().await` and then awaits the pinned `start`.

Dependencies: `kira-core`, `kira-config`, `serenity.workspace = true`, `tokio.workspace = true`.

- [ ] **Step 4: Run the bot tests to verify they pass**

Run: `cargo test -p kira-bot`
Expected: PASS (3 tests).

- [ ] **Step 5: Write the failing binary version test**

`crates/kira/tests/version.rs`:

```rust
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
```

- [ ] **Step 6: Run it to verify it fails**

Run: `cargo test -p kira --test version`
Expected: FAIL — package `kira` does not exist.

- [ ] **Step 7: Implement the binary and feature flag**

`crates/kira/Cargo.toml`:

```toml
[package]
name = "kira"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[features]
default = ["server"]
server = ["dep:kira-server"]

[dependencies]
kira-bot = { path = "../kira-bot" }
kira-config = { path = "../kira-config" }
kira-core = { path = "../kira-core" }
kira-server = { path = "../kira-server", optional = true }
tokio.workspace = true
```

`main.rs` uses `#[tokio::main]`, parses the first argument (`--version`, `--help`, or unknown), then loads config and calls `app::run_all`. Put the composition in `app.rs`:

- `async fn wait_flag(mut rx: tokio::sync::watch::Receiver<bool>)` waits until the value is `true` or the sender is dropped.
- Server build: `run_all(discord: DiscordConfig, server: ServerConfig)` pins `kira_bot::run(discord, wait_flag(rx.clone()))` and `kira_server::serve(&server, wait_flag(rx.clone()))`, selects over both plus `kira_core::shutdown::wait_for_signal()`, sends `true` on the watch channel, then awaits both pinned futures.
- Bot-only build (`#[cfg(not(feature = "server"))]`): same shape without the server future.

The `--version` path must print `kira <version>` and return before any config or network access.

- [ ] **Step 8: Run the binary tests and both builds**

Run: `cargo test -p kira --test version`
Expected: PASS (2 tests).

Run: `cargo build -p kira`
Expected: success.

Run: `cargo build -p kira --no-default-features`
Expected: success, and `cargo tree -p kira --no-default-features | grep kira-server` prints nothing.

- [ ] **Step 9: Commit**

```bash
git add Cargo.lock crates/kira-bot crates/kira
git commit -m "feat: restore bot runtime and add kira binary with server feature"
```

---

### Task 4: OpenAPI Capstone and xtask

**Files:**
- Create: `crates/kira-api-docs/Cargo.toml`, `crates/kira-api-docs/src/lib.rs`
- Create: `xtask/Cargo.toml`, `xtask/src/lib.rs`, `xtask/src/main.rs`, `xtask/src/openapi.rs`
- Create: `.cargo/config.toml`
- Create: `openapi/openapi.yaml` (generated)
- Modify: `Cargo.toml` (add `"xtask"` to members)

**Interfaces:**
- Consumes: `kira_server::health`, `kira_protocol::HealthResponse`.
- Produces:
  - `kira_api_docs::ApiDoc` (the `#[derive(OpenApi)]` type)
  - `kira_api_docs::yaml() -> Result<String, Box<dyn std::error::Error>>`
  - `xtask::openapi::generate(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>>`
  - `xtask::openapi::check(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>>`

- [ ] **Step 1: Write the failing xtask tests**

`xtask/tests/openapi.rs`:

```rust
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
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p xtask`
Expected: FAIL — package `xtask` does not exist.

- [ ] **Step 3: Implement `kira-api-docs`**

Dependencies: `kira-server`, `kira-protocol`, `utoipa = { workspace = true, features = ["yaml"] }`.

The derive must reference the crate-root handler path validated by the design spike:

```rust
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(title = "Kira API", version = env!("CARGO_PKG_VERSION"), description = "Kira service API"),
    tags((name = "health", description = "Service health")),
    paths(kira_server::health),
    components(schemas(kira_protocol::HealthResponse))
)]
pub struct ApiDoc;

pub fn yaml() -> Result<String, Box<dyn std::error::Error>> {
    Ok(ApiDoc::openapi().to_yaml()?)
}
```

- [ ] **Step 4: Implement `xtask`**

Add `"xtask"` to workspace members. `.cargo/config.toml`:

```toml
[alias]
xtask = "run --package xtask --"
```

`xtask` has a lib (`src/lib.rs` exposing `pub mod openapi;`) so integration tests can call it. Dependencies: `kira-api-docs`. Dev-dependency: `tempfile`.

`generate(root)` builds `root.join("openapi/openapi.yaml")`, creates the directory, trims any trailing newline from `yaml()`, writes exactly one trailing `\n`.

`check(root)` reads the committed file, regenerates the same bytes, and returns `Err` with a message naming the file and `cargo xtask openapi generate` when missing or different. It never writes.

`main.rs` accepts exactly `openapi generate` or `openapi check`; anything else prints usage and exits 2. The root for the CLI is `Path::new(env!("CARGO_MANIFEST_DIR"))` (the xtask directory is the repository root).

- [ ] **Step 5: Run the xtask tests to verify they pass**

Run: `cargo test -p xtask`
Expected: PASS (4 tests).

- [ ] **Step 6: Generate and check the committed contract**

Run: `cargo xtask openapi generate`
Run: `cargo xtask openapi check`
Expected: file created and check exits 0.

Run: `printf '\n# drift\n' >> openapi/openapi.yaml`
Run: `cargo xtask openapi check`
Expected: non-zero exit with the regeneration hint.

Run: `cargo xtask openapi generate`
Expected: drift removed; check passes again.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock .cargo crates/kira-api-docs xtask openapi
git commit -m "feat: add utoipa openapi capstone and xtask generate/check"
```

---

### Task 5: Vite + React Dashboard

**Files:**
- Create: `package.json`
- Create: `apps/dashboard/package.json`, `apps/dashboard/index.html`, `apps/dashboard/vite.config.ts`, `apps/dashboard/tsconfig.json`
- Create: `apps/dashboard/src/main.tsx`, `apps/dashboard/src/App.tsx`, `apps/dashboard/src/vite-env.d.ts`
- Create: `package-lock.json` (generated, committed)

**Interfaces:**
- Consumes: nothing (the health call is plain `fetch`).
- Produces:
  - `npm run build -w @kira/dashboard` writes `apps/dashboard/dist/index.html` containing `<div id="root">`.
  - Dev proxy forwards `/api` and `/ws` to `http://127.0.0.1:3001`.

- [ ] **Step 1: Write the root workspace manifest and verify the build fails**

Root `package.json`:

```json
{
  "name": "kira",
  "private": true,
  "workspaces": ["apps/*", "packages/*"],
  "scripts": {
    "build": "npm run build -w @kira/dashboard",
    "typecheck": "npm run typecheck --workspaces --if-present"
  }
}
```

Run: `npm run build -w @kira/dashboard`
Expected: FAIL — the dashboard workspace does not exist.

- [ ] **Step 2: Create the dashboard package and install**

`apps/dashboard/package.json`:

```json
{
  "name": "@kira/dashboard",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "typecheck": "tsc --noEmit"
  }
}
```

Run:

```bash
npm install -w @kira/dashboard react react-dom
npm install -w @kira/dashboard -D vite @vitejs/plugin-react typescript @types/react @types/react-dom
```

- [ ] **Step 3: Add the Vite, TS, and placeholder app files**

`tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noEmit": true,
    "skipLibCheck": true,
    "types": ["vite/client"]
  },
  "include": ["src", "vite.config.ts"]
}
```

`vite.config.ts` sets `build.outDir = "dist"` and `build.emptyOutDir = true`, and proxies both paths to the local CLI:

```ts
server: {
  proxy: {
    "/api": "http://127.0.0.1:3001",
    "/ws": { target: "ws://127.0.0.1:3001", ws: true }
  }
}
```

`index.html` contains `<div id="root"></div>`. `main.tsx` mounts `<App />` into it. `App.tsx` renders `<h1>Kira Dashboard</h1>`, fetches `/api/health` on mount, and displays `Server: <status>` or `Server: offline`. No routing or product UI.

- [ ] **Step 4: Build and verify the output**

Run: `npm run build -w @kira/dashboard`
Expected: success.

Run: `grep -q '<div id="root">' apps/dashboard/dist/index.html`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add package.json package-lock.json apps/dashboard
git commit -m "feat: add vite react dashboard placeholder"
```

---

### Task 6: Remote Client Library

**Files:**
- Create: `crates/kira-client/Cargo.toml`, `crates/kira-client/src/lib.rs`
- Create: `crates/kira-client/tests/http.rs`, `crates/kira-client/tests/ws.rs`

**Interfaces:**
- Consumes: `kira_protocol::{HealthResponse, ErrorResponse}`, `kira_core::Result`.
- Produces:
  - `#[derive(Clone)] pub struct Client { /* reqwest::Client + base URL */ }`
  - `kira_client::Client::new(base_url: impl Into<String>) -> Self`
  - `async fn Client::health(&self) -> Result<HealthResponse, ClientError>`
  - `async fn Client::connect_ws(&self, path: &str) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, ClientError>`
  - Re-exports: `Message`, `WebSocketStream`, `MaybeTlsStream`
  - `ClientError` variants: `Http(#[from] reqwest::Error)`, `Ws(#[from] tokio_tungstenite::tungstenite::Error)`, `InvalidUrl(String)`

- [ ] **Step 1: Write the failing HTTP tests**

`crates/kira-client/tests/http.rs` spawns a local Axum app that returns `Json(HealthResponse::ok())` at `/health` on `127.0.0.1:0`, then:

```rust
#[tokio::test]
async fn health_returns_the_body() {
    let (client, _server) = spawn_health().await;
    let health = client.health().await.unwrap();
    assert_eq!(health.status, "ok");
}

#[tokio::test]
async fn health_maps_http_errors() {
    let (client, _server) = spawn_status(500).await;
    assert!(matches!(client.health().await, Err(kira_client::ClientError::Http(_))));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p kira-client --test http`
Expected: FAIL — package does not exist.

- [ ] **Step 3: Implement the client**

Dependencies: `kira-core`, `kira-protocol`, `reqwest.workspace = true`, `thiserror.workspace = true`, `tokio-tungstenite.workspace = true`. Dev-dependencies: `axum.workspace = true`, `futures-util`, `tokio.workspace = true`.

`health()` builds `format!("{}/health", self.base_url.trim_end_matches('/'))`, sends with `reqwest::Client`, calls `error_for_status()`, and deserializes JSON.

`connect_ws(path)` derives the URL with a pure helper:

```rust
fn ws_url(base: &str, path: &str) -> Result<String, ClientError>
```

Parse with `reqwest::Url`; map `http`→`ws` and `https`→`wss`; any other scheme is `InvalidUrl`; ensure `path` starts with `/`; return the string. Then `connect_async(...).await` and return the stream.

Add unit tests for `ws_url`: `http://x:1` + `/ws` → `ws://x:1/ws`, and `https://x` + `/ws` → `wss://x/ws`.

- [ ] **Step 4: Run the HTTP and URL tests to verify they pass**

Run: `cargo test -p kira-client`
Expected: PASS.

- [ ] **Step 5: Write and pass the failing WebSocket test**

`tests/ws.rs` spawns an Axum echo app on `127.0.0.1:0`, calls `client.connect_ws("/ws")`, sends `Message::Text("hello".into())`, and asserts the echoed frame. Run it before the WS path is complete to see it fail, then implement and rerun.

Run: `cargo test -p kira-client --test ws`
Expected after implementation: PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.lock crates/kira-client
git commit -m "feat: add remote http and websocket client library"
```

---

### Task 7: Local CLI Server with Embedded Dashboard

**Files:**
- Create: `crates/kira-cli/Cargo.toml`, `crates/kira-cli/build.rs`
- Create: `crates/kira-cli/src/lib.rs`, `crates/kira-cli/src/main.rs`, `crates/kira-cli/src/assets.rs`, `crates/kira-cli/src/proxy.rs`
- Create: `crates/kira-cli/tests/assets.rs`, `crates/kira-cli/tests/proxy.rs`

**Interfaces:**
- Consumes: `kira_client::Client`, `kira_config::CliConfig`, `kira_core::{shutdown, Result}`, `kira_protocol::ErrorResponse`.
- Produces:
  - `#[derive(Clone)] pub struct AppState { pub client: kira_client::Client }`
  - `kira_cli::router(state: AppState) -> axum::Router`
  - `kira_cli::assets::{get, content_type_for, cache_control_for}`
  - Binary `kira-cli` that serves the local router on `CliConfig::bind`.

Dependencies: `axum.workspace = true`, `futures-util.workspace = true`, `kira-client`, `kira-config`, `kira-core`, `kira-protocol`, `mime_guess.workspace = true`, `rust-embed.workspace = true`, `tokio.workspace = true`, `tokio-tungstenite.workspace = true`. Dev-dependencies: `http-body-util`, `serde_json`, `tower`.

- [ ] **Step 1: Write the failing asset and API routing tests**

`crates/kira-cli/tests/assets.rs` builds `router(AppState { client: Client::new("http://127.0.0.1:1") })` and uses `tower::ServiceExt::oneshot`:

```rust
#[tokio::test]
async fn index_is_served() {
    let res = get("/").await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "text/html");
    assert!(body_string(res).await.contains("<div id=\"root\">"));
}

#[tokio::test]
async fn spa_fallback_serves_index() {
    assert_eq!(get("/dashboard/settings").await.status(), StatusCode::OK);
}

#[tokio::test]
async fn missing_asset_is_404() {
    assert_eq!(get("/assets/does-not-exist.js").await.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn api_unknown_path_is_404() {
    assert_eq!(get("/api/does-not-exist").await.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn traversal_does_not_escape() {
    let body = body_string(get("/../../etc/passwd").await).await;
    assert!(!body.contains("root:"));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p kira-cli --test assets`
Expected: FAIL — package does not exist.

- [ ] **Step 3: Implement `build.rs`**

```rust
use std::{env, fs, path::Path};

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
```

`PLACEHOLDER` is a minimal HTML document containing `<div id="root">` and the text `Dashboard not built`.

- [ ] **Step 4: Implement embedded asset serving**

`assets.rs`:

```rust
#[derive(rust_embed::RustEmbed)]
#[folder = "../../apps/dashboard/dist/"]
struct Dashboard;

pub fn get(path: &str) -> Option<rust_embed::EmbeddedFile> { Dashboard::get(path) }
pub fn content_type_for(path: &str) -> String {
    mime_guess::from_path(path).first_or_octet_stream().to_string()
}
pub fn cache_control_for(path: &str) -> &'static str {
    if path.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" }
}
```

Add unit tests for `content_type_for("app.js")` containing `javascript` and `cache_control_for("assets/app.js")` being the immutable value.

- [ ] **Step 5: Implement the router and run the asset tests**

Router behavior:

- `GET /` serves `index.html`.
- Any path not starting with `api/` and not routed explicitly serves `index.html` (SPA fallback).
- Paths starting with `assets/` that are missing return 404.
- `/api/*` misses return 404, never `index.html`.
- Lookups use exact embedded paths; never join a filesystem root.
- `Content-Type` comes from `content_type_for`; `Cache-Control` from `cache_control_for`.

Run: `cargo test -p kira-cli --test assets`
Expected: PASS (5 integration tests + 2 unit tests).

- [ ] **Step 6: Write the failing HTTP proxy tests**

`crates/kira-cli/tests/proxy.rs`:

- Spawn an upstream Axum server returning `Json(HealthResponse::ok())`; point `Client` at it; assert `GET /api/health` returns 200 and body `status == "ok"`.
- Point `Client` at `http://127.0.0.1:1`; assert `GET /api/health` returns 502 and a JSON `ErrorResponse`.

Run: `cargo test -p kira-cli --test proxy`
Expected: FAIL — route missing.

- [ ] **Step 7: Implement the HTTP proxy and rerun**

`GET /api/health` calls `state.client.health().await`; on `Ok`, return `Json(health)`; on `Err`, return `(StatusCode::BAD_GATEWAY, Json(ErrorResponse { error: e.to_string() }))`.

Run: `cargo test -p kira-cli --test proxy`
Expected: PASS.

- [ ] **Step 8: Write and pass the WebSocket proxy test**

Spawn an upstream Axum echo server, start the CLI router on `127.0.0.1:0`, connect a `tokio-tungstenite` client to `/ws`, send `Text("hello")`, assert the echoed text. Run before implementing to see failure, then implement.

`proxy.rs` bridges frames explicitly:

```rust
fn to_tungstenite(m: axum::extract::ws::Message) -> tokio_tungstenite::tungstenite::Message
fn to_axum(m: tokio_tungstenite::tungstenite::Message) -> axum::extract::ws::Message
```

Map Text, Binary, Ping, Pong, and Close; use `Vec<u8>`/`String` conversions so the two crates' payload types do not need to match. Add a round-trip unit test for each variant. Use `futures_util::{SinkExt, StreamExt}` and split both sockets; propagate close and errors; do not parse payloads.

Run: `cargo test -p kira-cli --test proxy`
Expected: PASS.

- [ ] **Step 9: Implement the binary and verify both asset modes**

`main.rs` loads `CliConfig::from_env()`, builds `Client::new(config.remote_url.clone())`, binds `config.bind`, and serves `router(AppState { client })` with `kira_core::shutdown::wait_for_signal()` as graceful shutdown.

Verify the release-mode guard:

```bash
mv apps/dashboard/dist /tmp/kira-dist-backup
cargo clean -p kira-cli
if cargo build --release -p kira-cli; then echo "FAIL: expected missing-asset failure"; exit 1; fi
mv /tmp/kira-dist-backup apps/dashboard/dist
```

Expected: the build fails with the `make dashboard` message.

- [ ] **Step 10: Commit**

```bash
git add Cargo.lock crates/kira-cli
git commit -m "feat: add kira-cli with embedded dashboard and remote proxies"
```

---

### Task 8: TypeScript API Types and Drift Check

**Files:**
- Create: `packages/api-types/package.json`, `packages/api-types/tsconfig.json`
- Create: `packages/api-types/src/index.ts`, `packages/api-types/src/schema.d.ts` (generated)
- Create: `packages/api-types/scripts/check.mjs`
- Modify: `package.json` (devDependency and scripts)
- Modify: `apps/dashboard/package.json`, `apps/dashboard/src/App.tsx`

**Interfaces:**
- Consumes: `openapi/openapi.yaml`.
- Produces:
  - `@kira/api-types` exporting the generated `components`, `paths`, and `operations`.
  - `npm run gen:api-types` rewrites `schema.d.ts`.
  - `npm run check:api-types` exits non-zero when it is stale.

- [ ] **Step 1: Verify generation fails before the package exists**

Run: `npm run gen:api-types`
Expected: FAIL — script does not exist.

- [ ] **Step 2: Create the package and generation scripts**

`packages/api-types/package.json`:

```json
{
  "name": "@kira/api-types",
  "private": true,
  "type": "module",
  "exports": {
    ".": "./src/index.ts",
    "./schema": "./src/schema.d.ts"
  },
  "scripts": {
    "gen": "openapi-typescript ../../openapi/openapi.yaml -o src/schema.d.ts",
    "check": "node scripts/check.mjs",
    "typecheck": "tsc --noEmit"
  }
}
```

`packages/api-types/tsconfig.json`:

```json
{
  "compilerOptions": {
    "strict": true,
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "noEmit": true,
    "skipLibCheck": true
  },
  "include": ["src"]
}
```

Install the generator at the root: `npm install -D openapi-typescript`.

Add root scripts:

```json
"gen:api-types": "npm run gen -w @kira/api-types",
"check:api-types": "npm run check -w @kira/api-types"
```

`src/index.ts` is `export * from "./schema";`.

- [ ] **Step 3: Implement the drift checker**

`scripts/check.mjs` resolves the repo root from `import.meta.url`, runs the local `node_modules/.bin/openapi-typescript` (`.cmd` on Windows) against `openapi/openapi.yaml` into a temp file, reads both files, and exits 1 with `schema.d.ts is stale; run npm run gen:api-types` when they differ. Delete the temp file in `finally`.

- [ ] **Step 4: Generate and verify the contract**

Run: `npm run gen:api-types`
Expected: `packages/api-types/src/schema.d.ts` is created.

Run: `npm run check:api-types`
Expected: exit 0.

Run:

```bash
cp packages/api-types/src/schema.d.ts /tmp/kira-schema.bak
printf '\n' >> packages/api-types/src/schema.d.ts
npm run check:api-types && exit 1 || true
cp /tmp/kira-schema.bak packages/api-types/src/schema.d.ts
```

Expected: the check exits non-zero, then the file is restored.

- [ ] **Step 5: Make the dashboard consume the types and typecheck**

Add `"@kira/api-types": "*"` to `apps/dashboard/package.json`, run `npm install`, and in `App.tsx`:

```tsx
import type { components } from "@kira/api-types";
type Health = components["schemas"]["HealthResponse"];
```

Use `Health` as the type of the parsed `/api/health` response.

Run: `npm run typecheck`
Expected: exit 0 for the dashboard and API types.

- [ ] **Step 6: Commit**

```bash
git add package.json package-lock.json packages/api-types apps/dashboard
git commit -m "feat: generate typescript api types from openapi and check drift"
```

---

### Task 9: Make Targets, Hygiene, and Full Verification

**Files:**
- Modify: `Makefile`
- Create: `.env.example`
- Modify: `scripts/smoke-test.sh`

**Interfaces:**
- Consumes: everything above.
- Produces: `make build`, `make release`, `make build-cli`, `make build-all`, `make install`, `make dashboard`, `make openapi`, `make test`, `make check`; `scripts/smoke-test.sh` exits 0.

- [ ] **Step 1: Verify `make check` fails before the targets exist**

Run: `make check`
Expected: FAIL — current Makefile has no `check` target.

- [ ] **Step 2: Write the Makefile**

```make
CARGO := cargo

.PHONY: all build dev release build-cli build-all install dashboard openapi test check clean fmt clippy

all: build

build:
	$(CARGO) build -p kira

dev:
	$(CARGO) watch -w crates -x "run -p kira"

release:
	$(CARGO) build --release -p kira

node_modules: package.json package-lock.json
	npm ci

install: node_modules

dashboard: node_modules
	npm run build -w @kira/dashboard

build-cli: dashboard
	$(CARGO) build --release -p kira-cli

build-all: dashboard
	$(CARGO) build --release -p kira -p kira-cli

openapi: node_modules
	$(CARGO) xtask openapi generate
	npm run gen:api-types

test:
	$(CARGO) test --workspace

check: node_modules
	$(CARGO) fmt --check
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	$(CARGO) test --workspace
	$(CARGO) xtask openapi check
	npm run check:api-types
	npm run typecheck
	@! $(CARGO) tree -p kira-cli | grep -q kira-server
	@! $(CARGO) tree -p kira-cli | grep -q serenity
	@! $(CARGO) tree -p kira | grep -q kira-api-docs
	@! $(CARGO) tree -p kira | grep -q xtask

fmt:
	$(CARGO) fmt

clippy:
	$(CARGO) clippy

clean:
	$(CARGO) clean
	rm -rf node_modules apps/dashboard/dist
```

- [ ] **Step 3: Write `.env.example` and update the smoke test**

`.env.example`:

```dotenv
DISCORD_TOKEN=
KIRA_BIND=127.0.0.1:3000
KIRA_CLI_BIND=127.0.0.1:3001
KIRA_REMOTE_URL=http://127.0.0.1:3000
```

`scripts/smoke-test.sh` keeps the release build and binary existence check, then:

```bash
OUTPUT="$("$BINARY" --version)"
if [[ "$OUTPUT" != kira\ * ]]; then
    echo "FAIL: unexpected --version output: $OUTPUT"
    exit 1
fi
echo "PASS: $OUTPUT"
```

- [ ] **Step 4: Run the full check**

Run: `make check`
Expected: exit 0. This runs formatting, clippy with `-D warnings`, all workspace tests, OpenAPI drift, TS drift, TypeScript typecheck, and the four dependency assertions.

- [ ] **Step 5: Run the acceptance commands**

```bash
make release
cargo build --release -p kira --no-default-features
make build-cli
bash scripts/smoke-test.sh
```

Expected: all succeed. Confirm the boundary assertions separately if `make check` was cached:

```bash
cargo tree -p kira-cli | grep -q kira-server && exit 1 || true
cargo tree -p kira-cli | grep -q serenity && exit 1 || true
cargo tree -p kira | grep -q kira-api-docs && exit 1 || true
cargo tree -p kira | grep -q xtask && exit 1 || true
```

- [ ] **Step 6: Commit**

```bash
git add Makefile .env.example scripts/smoke-test.sh
git commit -m "chore: add make targets, env example, and smoke test"
```
