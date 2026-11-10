# Rust Style Conformance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring every Rust crate in the Kira workspace into conformance with the project's Rust style rules without changing runtime behavior.

**Architecture:** Apply the guide's tool configuration workspace-wide, then clean up each crate in dependency order: leaves first (docs, `#[non_exhaustive]`, `#[must_use]`, local exceptions), libraries next, applications last. Finish by regenerating the OpenAPI and TypeScript contracts and running `make check`.

**Tech Stack:** Rust 2024 (MSRV 1.88, stable 1.99.1 active, nightly available for rustfmt), Cargo workspace lints, clippy, rustfmt, utoipa, axum, tokio.

**Spec:** `docs/superpowers/specs/` has no separate document for this change; the approved short design is the conversation scope confirmed on 2026-11-10. The rules source is the project's Rust style rules, applied as a workspace-wide configuration plus per-crate cleanup.

## Global Constraints

- Behavior must not change; this is a style and API-surface refactor only.
- Do not add dependencies or introduce a logging crate (`println!`/`eprintln!` stay).
- Every public item gets a doc comment that adds information, not one that restates its name.
- Exported enums and all-public-field structs get `#[non_exhaustive]`; add constructors where external code or integration tests need to build one.
- Use `cargo +nightly fmt` for the unstable rustfmt options. The active toolchain is stable 1.99.1 via `RUSTUP_TOOLCHAIN`; `cargo fmt --check` must still pass afterwards.
- Generated files are regenerated only with `make openapi`; never hand-edit `openapi/openapi.yaml` or `packages/api-types/src/schema.d.ts`.
- Final gate is `make check`.

## Review Focus

- Doc comments on `kira-protocol` DTO fields change generated OpenAPI descriptions and TypeScript types — regeneration is mandatory, not cosmetic.
- `#[non_exhaustive]` blocks struct-literal construction outside the defining crate — integration tests must move to constructors (`AppState::new`, `ErrorResponse::new`).
- A crate that omits `[lints] workspace = true` silently escapes clippy; verify all ten manifests.
- Moving `kira-bot`'s runtime into `bot.rs` must preserve the `lib.rs` public surface (`BotError`, `run`, `Handler`, `ConfigKey`, `is_ping`).
- Stable rustfmt ignores the unstable import options; run nightly once and verify both nightly and stable checks.

---

### Task 1: Tooling configuration and formatting

**Files:**
- Create: `rustfmt.toml`
- Create: `clippy.toml`
- Modify: `Cargo.toml`
- Modify: `crates/kira-core/Cargo.toml`, `crates/kira-config/Cargo.toml`, `crates/kira-protocol/Cargo.toml`
- Modify: `crates/kira-bot/Cargo.toml`, `crates/kira-server/Cargo.toml`, `crates/kira-client/Cargo.toml`
- Modify: `crates/kira/Cargo.toml`, `crates/kira-cli/Cargo.toml`, `crates/kira-api-docs/Cargo.toml`
- Modify: `xtask/Cargo.toml`

**Interfaces:**
- Consumes: nothing.
- Produces: workspace lint tables every crate inherits with `[lints] workspace = true`.

- [ ] **Step 1: Create `rustfmt.toml`**

```toml
imports_granularity = "Module"
hex_literal_case = "Lower"
format_macro_matchers = true
```

- [ ] **Step 2: Create `clippy.toml`**

```toml
avoid-breaking-exported-api = false
allow-unwrap-in-tests = true
disallowed-methods = [
  { path = "std::panic::catch_unwind", reason = "project style forbids catch_unwind in first-party code" },
  { path = "std::sync::PoisonError::into_inner", reason = "project style: never reach inside a poisoned mutex" },
  { path = "std::sync::PoisonError::get_ref", reason = "project style: never reach inside a poisoned mutex" },
  { path = "std::sync::PoisonError::get_mut", reason = "project style: never reach inside a poisoned mutex" },
]
```

- [ ] **Step 3: Add workspace lints to the root `Cargo.toml`**

Append these tables after `[workspace.dependencies]` (keep the existing tables intact):

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
unstable_features = "deny"
missing_docs = "warn"

[workspace.lints.clippy]
ref_option = "deny"
unwrap_used = "deny"
wildcard_imports = "deny"
needless_for_each = "deny"
undocumented_unsafe_blocks = "deny"
derive_partial_eq_without_eq = "allow"
missing_const_for_fn = "allow"
exhaustive_enums = "deny"
exhaustive_structs = "deny"
must_use_candidate = "deny"
return_self_not_must_use = "deny"
missing_panics_doc = "deny"
```

- [ ] **Step 4: Opt every member into the workspace lints**

Add this table to each of the ten `Cargo.toml` files listed above, after `[dev-dependencies]` (or after `[dependencies]` when there is no dev-dependencies table):

```toml
[lints]
workspace = true
```

- [ ] **Step 5: Format with nightly rustfmt and verify both toolchains**

Run: `cargo +nightly fmt`
Then: `cargo +nightly fmt --check`
Then: `cargo fmt --check`
Expected: all three commands exit 0; `use` blocks are merged at module granularity.

- [ ] **Step 6: Confirm the new lints are active**

Run: `cargo clippy -p kira-core --all-targets -- -D warnings`
Expected: FAIL with `missing documentation for the crate` / `missing documentation for a module` (proves the configuration is live; later tasks clear it).

- [ ] **Step 7: Commit**

```bash
git add rustfmt.toml clippy.toml Cargo.toml crates/*/Cargo.toml xtask/Cargo.toml
git commit -m "chore: adopt workspace Rust tool configuration"
```

---

### Task 2: Leaf crates — `kira-core`, `kira-config`, `kira-protocol`

**Files:**
- Modify: `crates/kira-core/src/lib.rs`
- Modify: `crates/kira-core/src/shutdown.rs`
- Modify: `crates/kira-config/src/lib.rs`
- Modify: `crates/kira-protocol/src/lib.rs`
- Modify: `openapi/openapi.yaml` (generated)
- Modify: `packages/api-types/src/schema.d.ts` (generated)

**Interfaces:**
- Consumes: Task 1 lint configuration.
- Produces: `ErrorResponse::new(error: impl Into<String>) -> Self`; `HealthResponse::ok()` and `ErrorResponse::new` are `#[must_use]`.

- [ ] **Step 1: Document and tighten `kira-core`**

Add `//! Signal handling for graceful shutdown.` to `lib.rs`, document `pub mod shutdown;`, add `//!` to `shutdown.rs`, and document `wait_for_signal`. Replace the local `use tokio::signal::unix::{SignalKind, signal};` with `use tokio::signal::unix;` and call `unix::signal(unix::SignalKind::terminate())?` (free functions go through their module).

- [ ] **Step 2: Document and tighten `kira-config`**

Remove the crate-level `#![allow(clippy::should_implement_trait)]` and its comment. Add crate docs plus docs for every config struct, field, `ConfigError` variant/field, `from_env`, and `from_iter`. Put this on each of the three inherent `from_iter` methods:

```rust
#[expect(
    clippy::should_implement_trait,
    reason = "fallible parsing cannot implement infallible FromIterator"
)]
```

Add `#[non_exhaustive]` to `ConfigError`, `DiscordConfig`, `ServerConfig`, and `CliConfig`. If clippy reports an unfulfilled `should_implement_trait` expectation, fall back to `#[allow(clippy::should_implement_trait, reason = "fallible parsing cannot implement infallible FromIterator")]`.

- [ ] **Step 3: Document and tighten `kira-protocol`**

Add crate docs and field docs for `HealthResponse` and `ErrorResponse`. Add `#[non_exhaustive]` to both structs. Add `#[must_use]` to `HealthResponse::ok`. Add:

```rust
impl ErrorResponse {
    /// Build an error payload from a human-readable message.
    #[must_use]
    pub fn new(error: impl Into<String>) -> Self {
        Self { error: error.into() }
    }
}
```

Use `ErrorResponse::new(...)` in the crate's own test instead of the struct literal. If `#[non_exhaustive]` breaks the `ToSchema` derive, replace it with `#[expect(clippy::exhaustive_structs, reason = "wire DTO fields are fixed by the API contract")]`.

- [ ] **Step 4: Verify the three leaf crates**

Run: `cargo clippy -p kira-core -p kira-config -p kira-protocol --all-targets -- -D warnings`
Expected: PASS.
Run: `cargo test -p kira-core -p kira-config -p kira-protocol`
Expected: PASS.

- [ ] **Step 5: Regenerate the contracts for the documented DTOs**

Run: `make openapi`
Expected: `openapi/openapi.yaml` and `packages/api-types/src/schema.d.ts` update with descriptions only; no route, field, or schema name disappears.

- [ ] **Step 6: Commit**

```bash
git add crates/kira-core crates/kira-config crates/kira-protocol \
  openapi/openapi.yaml packages/api-types/src/schema.d.ts
git commit -m "refactor: document and harden leaf crates"
```

---

### Task 3: `kira-client` and `kira-api-docs`

**Files:**
- Modify: `crates/kira-client/src/lib.rs`
- Modify: `crates/kira-client/tests/http.rs`
- Modify: `crates/kira-client/tests/ws.rs`
- Modify: `crates/kira-api-docs/src/lib.rs`

**Interfaces:**
- Consumes: Task 1 lint configuration.
- Produces: unchanged public function signatures.

- [ ] **Step 1: Document and tighten `kira-client`**

Keep the existing `//!` crate doc. Document the three `ClientError` variants, `Client`, `Client::new`, `health`, and `connect_ws`. Add `#[non_exhaustive]` to `ClientError`. Replace `use tokio_tungstenite::connect_async;` with a module import and call `tokio_tungstenite::connect_async(url)`. Add `//!` docs to `tests/http.rs` and `tests/ws.rs`.

- [ ] **Step 2: Document and tighten `kira-api-docs`**

Add crate docs, document `ApiDocsError`'s variant, `ApiDoc`, and `yaml()`. Add `#[non_exhaustive]` to `ApiDocsError`; add `#[non_exhaustive]` to `ApiDoc` unless the `#[derive(OpenApi)]` expansion rejects it, in which case use `#[expect(clippy::exhaustive_structs, reason = "OpenAPI marker type with no fields to grow")]`.

- [ ] **Step 3: Verify both crates**

Run: `cargo clippy -p kira-client -p kira-api-docs --all-targets -- -D warnings`
Expected: PASS.
Run: `cargo test -p kira-client -p kira-api-docs`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kira-client crates/kira-api-docs
git commit -m "refactor: document and harden client and docs crates"
```

---

### Task 4: `kira-bot` — module layout and conformance

**Files:**
- Create: `crates/kira-bot/src/bot.rs`
- Modify: `crates/kira-bot/src/lib.rs`
- Modify: `crates/kira-bot/src/handler.rs`

**Interfaces:**
- Consumes: Task 1 lint configuration.
- Produces: unchanged public surface, re-exported from the crate root:
  `pub use bot::{BotError, run};`
  `pub use handler::{ConfigKey, Handler, is_ping};`

- [ ] **Step 1: Move the runtime into `bot.rs`**

Move `BotError`, its `From<serenity::Error>` impl, `run`, `wait_for_gateway`, and the module's tests from `lib.rs` into `src/bot.rs`. Add `//!` docs to `bot.rs`, document `BotError` and its variant, and add `#[non_exhaustive]` to `BotError`.

- [ ] **Step 2: Reduce `lib.rs` to docs, modules, and re-exports**

`lib.rs` keeps only crate docs plus:

```rust
pub mod handler;

mod bot;

pub use bot::{BotError, run};
pub use handler::{ConfigKey, Handler, is_ping};
```

- [ ] **Step 3: Clean `handler.rs`**

Add `//!` module docs and docs for `ConfigKey`, `Handler`, and `is_ping`. Add `#[must_use]` to `is_ping`. Add `#[non_exhaustive]` to `ConfigKey` and `Handler` (marker structs; if that makes their re-exports useless, use `#[expect(clippy::exhaustive_structs, reason = "zero-sized marker type")]` instead). Replace `use serenity::{async_trait, model::channel::Message, prelude::*};` with explicit imports for the items used (`async_trait`, `Message`, `Context`, `EventHandler`, `TypeMapKey`). Make the boolean argument explicit at the call site: `is_ping(msg.content.as_str(), /* author_is_bot= */ msg.author.bot)`.

- [ ] **Step 4: Verify `kira-bot`**

Run: `cargo clippy -p kira-bot --all-targets -- -D warnings`
Expected: PASS.
Run: `cargo test -p kira-bot`
Expected: PASS, including both `wait_for_gateway` tests.

- [ ] **Step 5: Commit**

```bash
git add crates/kira-bot
git commit -m "refactor: split bot runtime module and document public surface"
```

---

### Task 5: `kira-server`

**Files:**
- Modify: `crates/kira-server/src/lib.rs`
- Modify: `crates/kira-server/tests/health.rs`
- Modify: `crates/kira-server/tests/ws.rs`

**Interfaces:**
- Consumes: Task 1 lint configuration, Task 2 DTOs.
- Produces: unchanged handler and router signatures.

- [ ] **Step 1: Document and tighten `kira-server`**

Add crate docs; document `ServerError` variants and their fields, `health`, `router`, `serve`, and `serve_with_listener`. Add `#[non_exhaustive]` to `ServerError`. Replace `use axum::routing::get;` with `use axum::routing;` and call `routing::get(health)` / `routing::get(ws_upgrade)` (free functions go through their module).

- [ ] **Step 2: Document the integration test crates**

Add `//!` docs to `tests/health.rs` and `tests/ws.rs`. In `tests/ws.rs`, import `tokio_tungstenite` as a module and call `tokio_tungstenite::connect_async(...)` instead of importing the free function.

- [ ] **Step 3: Verify `kira-server`**

Run: `cargo clippy -p kira-server --all-targets -- -D warnings`
Expected: PASS.
Run: `cargo test -p kira-server`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kira-server
git commit -m "refactor: document and harden server crate"
```

---

### Task 6: `kira-cli`, `kira`, and `xtask`

**Files:**
- Modify: `crates/kira-cli/src/lib.rs`
- Modify: `crates/kira-cli/src/assets.rs`
- Modify: `crates/kira-cli/src/dev_proxy.rs`
- Modify: `crates/kira-cli/src/proxy.rs`
- Modify: `crates/kira-cli/src/main.rs`
- Modify: `crates/kira-cli/build.rs`
- Modify: `crates/kira-cli/tests/assets.rs`, `tests/dev_proxy.rs`, `tests/proxy.rs`
- Modify: `crates/kira/src/main.rs`
- Modify: `crates/kira/tests/bind_failure.rs`, `tests/config.rs`, `tests/version.rs`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/openapi.rs`
- Modify: `xtask/src/main.rs`
- Modify: `xtask/tests/openapi.rs`

**Interfaces:**
- Consumes: Task 2 `ErrorResponse::new`, Task 3 `kira-client`.
- Produces: `AppState::new(client: kira_client::Client, dev_server_url: Option<String>) -> Self`.

- [ ] **Step 1: Add `AppState::new` and tighten `kira-cli/src/lib.rs`**

Add `#[non_exhaustive]` to `AppState`, document `pub client`, and add:

```rust
impl AppState {
    /// Build the shared state for the local dashboard server.
    #[must_use]
    pub fn new(client: kira_client::Client, dev_server_url: Option<String>) -> Self {
        Self {
            client,
            dev_server_url,
        }
    }
}
```

Update every `AppState { ... }` literal in `src/` and `tests/` to call `AppState::new(...)`.

- [ ] **Step 2: Add `#[must_use]` to dashboard asset helpers**

In `assets.rs`, add `#[must_use]` to `get`, `content_type_for`, and `cache_control_for`.

- [ ] **Step 3: Replace `ErrorResponse` struct literals**

In `dev_proxy.rs` and `proxy.rs`, replace `ErrorResponse { error: "..." .into() }` with `ErrorResponse::new("...")`.

- [ ] **Step 4: Document binary and test crate roots**

Add `//!` docs to `crates/kira-cli/src/main.rs`, `crates/kira-cli/build.rs`, all three `kira-cli` integration tests, `crates/kira/src/main.rs`, all three `kira` integration tests, `xtask/src/main.rs`, and `xtask/tests/openapi.rs`. In `crates/kira-cli/tests/proxy.rs`, call `tokio_tungstenite::connect_async(...)` through the module rather than importing the free function.

- [ ] **Step 5: Document `xtask`**

Add crate docs to `xtask/src/lib.rs`, `//!` docs to `xtask/src/openapi.rs`, and docs to `generate` and `check`.

- [ ] **Step 6: Verify the three packages**

Run: `cargo clippy -p kira-cli -p kira -p xtask --all-targets -- -D warnings`
Expected: PASS.
Run: `cargo test -p kira-cli -p kira -p xtask`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/kira-cli crates/kira xtask
git commit -m "refactor: document applications and add state constructor"
```

---

### Task 7: Regenerate contracts and run the full gate

**Files:**
- Modify: `openapi/openapi.yaml` (generated)
- Modify: `packages/api-types/src/schema.d.ts` (generated)

**Interfaces:**
- Consumes: all earlier tasks, especially DTO field docs.
- Produces: contracts in sync with the documented DTOs.

- [ ] **Step 1: Confirm the contracts are already current**

Run: `make openapi`
Expected: no diff in `openapi/openapi.yaml` or `packages/api-types/src/schema.d.ts` (Task 2 generated them).

- [ ] **Step 2: Run the authoritative gate**

Run: `make check`
Expected: PASS — fmt, clippy `-D warnings`, workspace tests, OpenAPI drift check, TypeScript type drift, typecheck, and dependency-boundary assertions.

- [ ] **Step 3: Fix any remaining findings and re-run**

If `make check` reports a finding, fix the source (never the generated file) and re-run `make check` until it passes.

- [ ] **Step 4: Commit any remaining changes**

```bash
git add -A
git commit -m "chore: finish Rust style conformance"
```

Skip the commit when `git status --short` is empty.
