# Kira Workspace Architecture — Design

- **Date:** 2026-11-08
- **Status:** Proposed
- **Scope:** Repository/workspace architecture, crate boundaries, Cargo features,
  OpenAPI generation, dashboard embedding, and build tooling. No application
  business logic.

## 1. Intent

Kira today is a single-crate Serenity Discord bot. The next stage needs three
things that pull in different directions:

1. A main service that runs the Discord bot and an HTTP/WebSocket API in one
   process.
2. A local CLI application that serves a bundled dashboard and talks to the
   remote service.
3. A reproducible contract between the Rust API and the TypeScript dashboard.

The goal of this change is to establish the workspace and the seams between
those parts so that feature work can proceed without re-litigating structure.
The repository should build each application independently, keep development
tooling out of production binaries, and make the OpenAPI document the single
source of truth for TypeScript types.

## 2. Current State

- One package at the repository root: `kira`.
- `src/main.rs` builds a multi-thread Tokio runtime, starts the Serenity
  gateway, and shuts down on Ctrl-C/SIGTERM.
- `src/config.rs` loads `DISCORD_TOKEN` via `dotenvy` + `envy`.
- `src/handler.rs` replies `Pong!` to `!ping`.
- `src/shutdown.rs` provides a cross-platform signal future.
- No Axum, Utoipa, frontend, workspace, or CI.
- Files are currently untracked in git, so the move is low-risk.
- `.env` contains a live-looking Discord token and is not gitignored.

## 3. Goals

- One Cargo workspace rooted at the repository.
- Rust code separated by ecosystem: `crates/` for crates, `xtask/` for tooling.
- Two independent application crates: `kira` and `kira-cli`.
- Reusable libraries with a one-directional dependency graph and no cycles.
- `kira` combines bot + HTTP/WS server; `server` is the default Cargo feature.
- `kira-cli` serves embedded production dashboard assets over a local Axum
  server and reaches the remote Kira server through `kira-client`.
- OpenAPI generated with Utoipa from the server route annotations.
- `openapi/openapi.yaml` generated, committed, and checked in CI/`make check`.
- TypeScript types generated from that YAML into `packages/api-types`.
- Production binaries do not contain OpenAPI assembly, YAML, `xtask`, or
  frontend build tooling.

## 4. Non-Goals

- Authentication, persistence, domain models, or business endpoints beyond the
  health/WebSocket scaffolding described here.
- A production dashboard UI. The dashboard is a placeholder that proves the
  asset pipeline and API wiring.
- A complete WebSocket protocol. The route and client plumbing are scaffolds.
- CI provider configuration (GitHub Actions, etc.). The Make targets and xtask
  commands are the interface; wiring CI is a follow-up.
- Logging/tracing infrastructure. The existing `println!`/`eprintln!` behavior
  is preserved for now.
- Publishing any crate. Every crate is `publish = false`.

## 5. Anti-Goals

- A single crate with modules. It would violate the independent-build
  requirement and make boundaries advisory.
- Multiple Cargo workspaces. It duplicates lockfiles, versions, and tooling.
- Putting `#[derive(OpenApi)]` or YAML serialization in `kira-server` behind a
  production feature. The document assembly belongs in a dev-only capstone.
- Committing `apps/dashboard/dist`. Build output is not source.
- Running `npm` implicitly from a Cargo build script. Builds should not acquire
  network or Node dependencies behind the developer's back.
- Re-exporting everything from a prelude or adding a `common`/`utils` crate.
  Shared code gets a real name or stays local.
- Using `cargo build --release --workspace` as the shipping build. Release
  targets name `-p kira` or `-p kira-cli` explicitly.
- Letting the committed TypeScript schema drift from the committed YAML.

## 6. Constraints

- Rust edition 2024, `resolver = "3"`.
- MSRV 1.88 (Utoipa 6's minimum). Current toolchain is 1.99.1.
- `axum` 0.8, `utoipa` 6, `utoipa-axum` 0.3 (compatible) — `utoipa-axum` is not
  used; plain `#[utoipa::path]` annotations keep the dependency surface smaller.
- `rust-embed` 8.13, `reqwest` 0.13, `tokio-tungstenite` 0.30.
- `typescript-openapi` does not exist. Use `openapi-typescript` 7.x.
- `openapi-typescript` reads YAML directly.
- The repository is currently untracked, so paths may move without history loss.

## 7. Approaches Considered

### 7.1 Repository topology

**A. Single crate with modules.** Simplest now, but cannot satisfy
`cargo build -p kira-cli`, cannot enforce that the CLI does not depend on the
bot, and makes the OpenAPI boundary advisory.

**B. One workspace, ecosystem-separated directories (chosen).** `crates/`,
`xtask/`, `apps/`, `packages/` under one root. One lockfile, one set of
workspace dependency versions, per-crate `-p` builds, one `cargo xtask`.

**C. Multiple workspaces.** Maximal isolation, but duplicates lockfiles and
tooling and makes cross-workspace `cargo xtask` awkward. Rejected.

### 7.2 Where the OpenAPI document is assembled

**A. `kira-server` depends on `kira-api-docs`.** Natural direction, but the
OpenAPI document and YAML serialization would be reachable from the production
binary. Rejected.

**B. `kira-api-docs` depends on `kira-server` (chosen).** A dev-only capstone.
It references `kira_server`'s public `#[utoipa::path]` handlers and schemas.
`xtask` depends on it; no application does. A two-crate compile spike confirmed
cross-crate `paths(kira_server::health)` works with Utoipa 6.

**C. Hand-written YAML.** Spec-first, but defeats the purpose of Utoipa and
drifts from the routes. Rejected.

### 7.3 Dashboard embedding

**A. `rust-embed` (chosen).** Release builds embed at compile time; debug
builds read from disk. Supports a clean SPA fallback and immutable asset
headers. Requires the dashboard build to exist for release builds.

**B. `include_dir`.** Also compile-time, but always embeds and does not give
the debug filesystem behavior. No advantage here.

**C. `tower-http::ServeDir` from disk.** No embedding; fails the requirement.

**D. `build.rs` generates a Rust asset module.** More control, but more custom
code and no benefit over `rust-embed`.

### 7.4 TypeScript generation

**A. `openapi-typescript` (chosen).** Maintained, reads YAML, emits `.d.ts`,
zero runtime dependency.

**B. `@hey-api/openapi-ts`.** Also viable, but generates a client SDK we do not
need. Rejected for now.

**C. Hand-written types.** Drift. Rejected.

### 7.5 Frontend package management

**A. npm workspaces (chosen).** No extra tooling; `apps/*` and `packages/*` in
the root `package.json`.

**B. pnpm workspaces.** Better disk/install behavior, but requires an extra
tool. Acceptable later; not needed for two packages.

## 8. Chosen Architecture

### 8.1 Tree

```
kira/
├── Cargo.toml                    # workspace only
├── Cargo.lock
├── .cargo/config.toml            # xtask alias
├── .env.example
├── .gitignore
├── Makefile
├── package.json                  # npm workspaces
├── openapi/
│   └── openapi.yaml              # generated + committed
├── crates/
│   ├── kira/                     # binary: bot + server
│   ├── kira-cli/                 # binary: local dashboard + remote client
│   ├── kira-core/                # errors, shutdown
│   ├── kira-config/              # env config
│   ├── kira-protocol/            # shared HTTP/WS DTOs
│   ├── kira-bot/                 # Serenity
│   ├── kira-server/              # Axum + utoipa annotations
│   ├── kira-client/              # reqwest + tokio-tungstenite
│   └── kira-api-docs/            # OpenApi derive + YAML (dev-only capstone)
├── xtask/                        # openapi generate/check
├── apps/
│   └── dashboard/                # Vite + React + TS
└── packages/
    └── api-types/                # generated schema.d.ts
```

`apps/` and `packages/` are excluded from the Cargo workspace; they are managed
by npm workspaces. `xtask/` is a Cargo workspace member.

### 8.2 Dependency graph

```
L0 leaves
  kira-core        kira-config        kira-protocol

L1 libraries
  kira-bot      -> kira-core, kira-config
  kira-server   -> kira-core, kira-config, kira-protocol
  kira-client   -> kira-core, kira-protocol

L2 applications
  kira          -> kira-bot, kira-server (optional), kira-config, kira-core
  kira-cli      -> kira-client, kira-config, kira-core

Dev-only sidecars
  kira-api-docs -> kira-server, kira-protocol
  xtask         -> kira-api-docs
```

Rules:

1. `core`, `config`, and `protocol` never depend on any other workspace crate.
2. `bot` and `server` never depend on each other.
3. `client` never depends on `bot` or `server`.
4. `kira-api-docs` is never a dependency of `kira`, `kira-cli`, `kira-bot`,
   `kira-server`, or `kira-client`.
5. `xtask` is never a dependency of any crate.
6. No cycles. A `cargo tree` check in `make check` enforces the important ones.

### 8.3 Crate responsibilities

| Crate | Owns | Does not own |
|---|---|---|
| `kira-core` | `shutdown::wait_for_signal`, common error aliases | config structs, HTTP, Discord |
| `kira-config` | env loading, `DiscordConfig`, `ServerConfig`, `CliConfig`, defaults | server state, client construction |
| `kira-protocol` | serde DTOs (`HealthResponse`), WS envelope types, `ToSchema` derives | transport, routing |
| `kira-bot` | Serenity client construction, event handler, `!ping` behavior | HTTP, config file parsing |
| `kira-server` | Axum router, `/health`, `/ws` scaffold, `serve`, `#[utoipa::path]` | bot, config loading, OpenAPI document |
| `kira-client` | HTTP `health()`, WebSocket `connect()` | server routing, dashboard assets |
| `kira-api-docs` | `#[derive(OpenApi)]`, tags/info, `to_yaml()` | any runtime application code |
| `kira` | process composition, feature gating, signal handling | reusable logic |
| `kira-cli` | embedded asset serving, local BFF routes, remote client wiring | bot, server crate, OpenAPI assembly |
| `xtask` | file generation/verification | anything shipped |

### 8.4 `kira` feature topology

```toml
[features]
default = ["server"]
server = ["dep:kira-server"]
```

- Default build: Discord bot + HTTP/WS server in one process.
- `--no-default-features`: bot only. No `kira-server`, no Axum in the tree.
- The bot is not feature-gated; the requirement only makes `server` a feature.
- `kira` must not depend on `kira-api-docs`, `xtask`, or any Node tooling.

### 8.5 `kira-cli` independence

- Depends on `kira-client`, `kira-config`, `kira-core`; never on `kira-server`,
  `kira-bot`, or Serenity.
- Serves the dashboard and proxies to the remote Kira server.
- Does not carry OpenAPI document assembly.

## 9. Configuration

Environment variables (loaded with `dotenvy` + `envy`):

| Variable | Used by | Default | Required |
|---|---|---|---|
| `DISCORD_TOKEN` | `kira` | — | yes |
| `KIRA_BIND` | `kira` server | `127.0.0.1:3000` | no |
| `KIRA_CLI_BIND` | `kira-cli` | `127.0.0.1:3001` | no |
| `KIRA_REMOTE_URL` | `kira-cli` | `http://127.0.0.1:3000` | no |

- `KIRA_REMOTE_URL` is the HTTP base. WebSocket URLs are derived:
  `http` → `ws`, `https` → `wss`.
- `.env.example` documents the variables without secrets.
- `.env` is gitignored. The currently exposed Discord token should be rotated.
- Loading `.env` is best effort (`dotenv().ok()`). A missing `.env` is **not**
  an error; deployments that inject environment variables directly must work.
  `envy` then validates required variables, so a missing `DISCORD_TOKEN` fails
  with a config error and a missing `KIRA_REMOTE_URL` falls back to its default.
- `DISCORD_TOKEN` is required only by `kira`, never by `kira-cli`.

Config types:

- `kira_config::DiscordConfig { token: String }`
- `kira_config::ServerConfig { bind: SocketAddr }`
- `kira_config::CliConfig { bind: SocketAddr, remote_url: String }` — kept as a
  string so `kira-config` does not need a URL crate; `kira-client` parses and
  validates it.

Each application loads only the config it needs. `Config::from_env()` for the
main binary; `CliConfig::from_env()` for the CLI.

## 10. HTTP and WebSocket Scaffolding

OpenAPI covers the remote Kira server only. The CLI's local routes are an
internal BFF and are not part of the published contract.

### `kira-server`

- `GET /health` → `200 application/json` with
  `kira_protocol::HealthResponse`.
  - Tag: `health`.
  - Annotated with `#[utoipa::path]`.
- `GET /ws` → WebSocket upgrade.
  - Minimal echo/close scaffold; no protocol messages are interpreted.
  - Not representable in OpenAPI; noted as intentionally undocumented.
- `pub fn router() -> axum::Router`
- `pub async fn serve(listener: TcpListener, shutdown: impl Future<Output = ()>)`
  with `axum::serve(...).with_graceful_shutdown(...)`.

### `kira-client`

- `Client::new(base_url) -> Result<Self>`
- `async fn health(&self) -> Result<HealthResponse>`
- `async fn connect_ws(&self, path: &str) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>>`
  - Returns the raw `tokio-tungstenite` stream and re-exports its `Message` type.
  - TLS uses `tokio-tungstenite`'s `rustls-tls-webpki-roots` feature; plain `ws`
    works without any TLS setup.
- `reqwest` 0.13 already defaults to rustls for HTTPS, plus the `json` feature.
- Typed `thiserror` error.

### `kira-cli` local server

- `GET /` and non-`/api` paths → embedded `index.html` (SPA fallback).
- `GET /assets/*` → embedded asset; `Cache-Control: public, max-age=31536000,
  immutable`; 404 if missing.
- `GET /api/health` → calls `kira-client::health()`; returns the remote JSON, or
  `502` with a JSON error if the remote is unreachable.
- `GET /ws` → upgrades and proxies frames to the remote `KIRA_REMOTE_URL` `/ws`
  without interpreting them. Scaffold only.
  - Axum's `WebSocket` yields `axum::extract::ws::Message`, while
    `kira-client` returns a `tokio-tungstenite` stream with a different
    `Message` type. `kira-cli` takes a **direct** `tokio-tungstenite`
    dependency at the same workspace version and converts frames explicitly in
    both directions. It uses `futures-util` to split both streams.
  - Close frames and errors are propagated; payloads are never parsed.
  - Keeping the conversion in `kira-cli` avoids making `kira-client` depend on
    Axum and avoids a transport-abstraction layer.
- `GET /api/nonexistent` must not fall back to `index.html`; it returns 404.

TLS: `https`/`wss` remote URLs are supported by the selected rustls features,
but acceptance testing is localhost HTTP/WS only in this phase.

### `kira`

- Load config; start the bot.
- If `feature = "server"`, bind `KIRA_BIND` and serve concurrently.
- On Ctrl-C/SIGTERM, gracefully stop the server and the Serenity shards, then
  exit.
- Support `--version` (`kira <version>`) so the existing smoke test does not
  start the bot and hang. `--help` may print usage; not required.

## 11. OpenAPI Workflow

### Ownership

- Route annotations and `ToSchema` derives live with their types/routes:
  `kira-server` and `kira-protocol`.
- Document assembly lives only in `kira-api-docs`:
  - `#[derive(OpenApi)]` with `paths(...)`, `components(schemas(...))`, tags,
    and info (title `Kira API`, version from the workspace).
  - `pub fn yaml() -> Result<String>` using Utoipa's `yaml` feature.
- `xtask` is the only consumer.

### Commands

```bash
cargo xtask openapi generate   # write openapi/openapi.yaml
cargo xtask openapi check      # regenerate in memory; byte-compare; exit 1 on drift
```

- `generate` writes a trailing newline and uses the serialization order Utoipa
  produces. The output must be stable across runs.
- `check` never writes. On drift it prints the path, a short diff hint, and
  tells the developer to run `generate`.
- `make openapi` runs `generate`, then `npm run gen:api-types`.
- Both files are committed:
  - `openapi/openapi.yaml`
  - `packages/api-types/src/schema.d.ts`

### How "no tooling in production binaries" is satisfied

- `utoipa` macros remain in `kira-server`/`kira-protocol` because the
  annotations are compile-time and add no runtime dependency.
- The `OpenApi` document, YAML serialization, `kira-api-docs`, and `xtask` are
  absent from every application's dependency tree.
- `kira` does not depend on `utoipa-axum`; path annotations are plain attributes.

## 12. Dashboard and Asset Embedding

### `apps/dashboard`

- Vite + React + TypeScript.
- `npm run build` outputs `apps/dashboard/dist/`.
- Dev server proxies `/api` and `/ws` to `kira-cli` (`127.0.0.1:3001`).
- Placeholder page calls `/api/health` and displays the result. No product UI.
- Package name: `@kira/dashboard`.

### Embedding

- `kira-cli` derives `RustEmbed` with
  `#[folder = "../../apps/dashboard/dist/"]` (relative to the crate manifest).
- Release builds embed the files at compile time.
- Debug builds read from disk, so rebuilding the dashboard is enough to refresh;
  no Rust rebuild needed.
- `build.rs` behavior:
  - If `dist/index.html` is missing in a **release** build, fail with an
    actionable message: `run make dashboard`.
  - If missing in a **debug** build, create a placeholder `dist/index.html`
    marked with `dist/.kira-placeholder`. The placeholder page explains that the
    dashboard has not been built.
  - A real Vite build removes the marker (empty-out-dir). If the marker exists
    during a release build, fail the same way.
  - Emit `cargo:rerun-if-changed` for `dist/`.
- Serving:
  - `index.html` and SPA fallbacks: `Cache-Control: no-cache`.
  - Hashed assets under `/assets/`: immutable, one-year cache.
  - MIME type by extension with `mime_guess`.
  - Path traversal is impossible because lookups go through the embedded file
    set by exact path, not the filesystem.

### Why the build-order constraint is acceptable

`cargo build --release -p kira-cli` is independent of the `kira` application and
of the bot/server crates, but it requires the dashboard output. The alternative
—auto-running npm from `build.rs`—adds hidden network/toolchain dependencies to
a Cargo build. `make dashboard` (or `make build-cli`) owns that ordering.

## 13. Frontend Packages

Root `package.json`:

- `private: true`
- workspaces: `apps/*`, `packages/*`
- `devDependencies`: `openapi-typescript`
- scripts:
  - `build` → dashboard build
  - `gen:api-types` → regenerate `packages/api-types`
  - `typecheck` → all workspaces with a typecheck script

`packages/api-types` (`@kira/api-types`):

- `src/schema.d.ts` — generated by `openapi-typescript` from
  `../../openapi/openapi.yaml`.
- `src/index.ts` — re-exports the generated types.
- `package.json` exports `./schema` and `./` for TS consumers.
- No bundling step; consumers compile the source.
- Generated file is committed so TypeScript builds do not require the Rust
  toolchain.
- `scripts/check.mjs` regenerates the schema into a temp file and byte-compares
  it with the committed one. It exits non-zero with a diff hint when stale, so
  YAML changes cannot silently leave the TypeScript contract behind.
- Scripts: `gen` (write committed file), `check` (drift check), `typecheck`.

The root `package-lock.json` is committed. `openapi-typescript` is pinned by the
lockfile so generation is reproducible.

`apps/dashboard` consumes `@kira/api-types` via the workspace protocol/name.

## 14. Build Tooling

### Root `Cargo.toml`

- `members = ["crates/*", "xtask"]`, `resolver = "3"`.
- `[workspace.package]`: version, edition, rust-version, `publish = false`.
- `[workspace.dependencies]` centralizes versions.
- `[profile.release]`: `opt-level = 3`, `codegen-units = 1`, `lto = "fat"`,
  `panic = "abort"`, `strip = true`.
- `[profile.dev]`: `debug = "line-tables-only"`, `split-debuginfo = "unpacked"`.
- Profiles must live at the workspace root; package-level profiles are ignored.

### `.cargo/config.toml`

```toml
[alias]
xtask = "run --package xtask --"
```

### Workspace build contract

The workspace is virtual, so a bare `cargo build` / `cargo build --release`
builds **every** member, including `kira-api-docs` (which enables Utoipa's YAML
feature) and `kira-cli` (which needs dashboard assets). That is not the
production path, and it weakens the "no OpenAPI assembly in production"
guarantee through feature unification. Make targets therefore name packages:

| Target | Command |
|---|---|
| `build` | `cargo build -p kira` |
| `release` | `cargo build --release -p kira` |
| `build-cli` | dashboard, then `cargo build --release -p kira-cli` |
| `build-all` | dashboard, then both release binaries |
| `test` | `cargo test --workspace` (debug; `kira-cli/build.rs` supplies the placeholder) |
| `check` | boundary, drift, and lint checks (below) |

- `make release` must not build `kira-cli`; the existing smoke test calls
  `make release` and then runs `target/release/kira`.
- Never use `cargo build --release --workspace` to produce shippable binaries:
  feature unification can pull `utoipa`'s YAML feature into `kira`, and the CLI
  needs Node output.
- `make dev` remains a `cargo watch` loop for the main binary.

### Node dependencies

- Commit `package-lock.json`.
- `make install` runs `npm ci`.
- `node_modules` is a Make prerequisite for `dashboard`, `openapi`, and
  `check`, so a fresh checkout bootstraps automatically.
- Network access is required only for the install step.

### Make targets

Preserve the existing `all`, `build`, `dev`, `release`, `test`, `clean`, `fmt`,
`clippy` targets and add:

- `install` → `npm ci`.
- `dashboard` → `install`, then `npm run build`.
- `build-cli` → `dashboard`, then `cargo build --release -p kira-cli`.
- `openapi` → `install`, `cargo xtask openapi generate`, then
  `npm run gen:api-types`.
- `check` → `install`, fmt check, clippy, tests, `cargo xtask openapi check`,
  `npm run check -w @kira/api-types`, frontend typecheck, and the
  dependency-boundary assertions below.

Boundary assertions:

```bash
! cargo tree -p kira-cli | grep -q kira-server
! cargo tree -p kira-cli | grep -q serenity
! cargo tree -p kira    | grep -q kira-api-docs
! cargo tree -p kira    | grep -q xtask
```

## 15. Security and Repository Hygiene

- `.gitignore`: `/target`, `.env`, `.env.*` (except `.env.example`),
  `node_modules/`, `apps/dashboard/dist/`.
- Add `.env.example` with empty/placeholder values.
- Do not commit the existing `.env`.
- Rotate the Discord token that has been exposed.

## 16. Migration of Existing Code

- Root package `kira` → `crates/kira` (binary only).
- `src/config.rs` → `crates/kira-config`.
- `src/shutdown.rs` → `crates/kira-core/src/shutdown.rs`.
- `src/handler.rs` and Serenity setup → `crates/kira-bot`.
- Root `Cargo.toml` becomes workspace-only; profiles move to the root.
- The `!ping` behavior is preserved.
- `scripts/smoke-test.sh` is updated to exercise `--version` without starting
  the gateway.

## 17. Acceptance Criteria

The change is done when all of these hold:

1. `cargo build --release -p kira` succeeds and produces a bot + server binary.
2. `cargo build --release -p kira --no-default-features` succeeds without Axum.
3. `cargo build --release -p kira-cli` succeeds after `make dashboard`.
4. `cargo tree -p kira-cli` contains neither `kira-server`, `kira-bot`, nor
   `serenity`.
5. `cargo tree -p kira` contains neither `kira-api-docs` nor `xtask`.
6. `cargo xtask openapi generate` produces a committed, stable
   `openapi/openapi.yaml`.
7. `cargo xtask openapi check` exits 0 on a clean tree and non-zero after an
   annotation-only change.
8. `npm run gen:api-types` regenerates `packages/api-types/src/schema.d.ts`;
   `npm run check -w @kira/api-types` fails when the committed file is stale.
9. `npm run build` produces `apps/dashboard/dist/`, and a release `kira-cli`
   serves it at `/` with SPA fallback and immutable asset headers.
10. `kira-cli` proxies `GET /api/health` to the remote server.
11. `cargo fmt --check` and `cargo clippy --workspace --all-targets
    --all-features -- -D warnings` pass.
12. No file under `crates/` or `xtask/` depends on a frontend path at compile
    time except `kira-cli/build.rs` and the `RustEmbed` folder.
13. `kira` starts with `DISCORD_TOKEN` supplied only through the environment
    (no `.env` file present).
14. `make release` produces `target/release/kira` without building
    `kira-cli`, `kira-api-docs`, or the dashboard.

## 18. What Is Left to the Builder

- Exact `HealthResponse` field names (proposal: `status`, `version`).
- Exact placeholder dashboard markup and styling.
- Whether `--help` is implemented alongside `--version`.
- The precise WS echo behavior and close codes.
- CI provider wiring and caching.
- Choosing whether to add a `rust-toolchain.toml` (the MSRV is pinned in the
  manifest either way).
- Test layout (unit tests per library vs. integration tests in the binaries).

## 19. Decision Log

| # | Decision | Status |
|---|---|---|
| 1 | One workspace, `crates/` + `xtask/` + `apps/` + `packages/` | Decided |
| 2 | `kira-api-docs` depends on `kira-server`, never the reverse | Decided |
| 3 | `kira` features: `default = ["server"]`; bot always compiled | Decided |
| 4 | `rust-embed` for dashboard assets; release embeds, debug reads disk | Decided |
| 5 | `openapi-typescript` replaces the non-existent `typescript-openapi` | Decided |
| 6 | npm workspaces | Decided |
| 7 | Release `kira-cli` requires dashboard `dist/`; debug uses a placeholder | Decided |
| 8 | OpenAPI documents the remote server only, not the CLI BFF | Decided |
| 9 | No `utoipa-axum`; plain `#[utoipa::path]` annotations | Decided |
| 10 | No logging/tracing crate in this phase | Decided |
| 11 | Make targets name packages; `make release` builds only `-p kira` | Decided |
| 12 | `make check` fails on stale `schema.d.ts`, not just stale YAML | Decided |
| 13 | `package-lock.json` committed; `npm ci` via `make install` | Decided |
| 14 | Missing `.env` tolerated; real environment variables are authoritative | Decided |
| 15 | WS frame conversion lives in `kira-cli`; direct `tokio-tungstenite` dep | Decided |
| 16 | rustls for `https`/`wss`; acceptance tests remain localhost HTTP/WS | Decided |
