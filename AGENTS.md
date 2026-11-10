# AGENTS.md

Kira is a Rust workspace plus npm workspaces. It produces two binaries: `kira` (Discord bot + HTTP/WebSocket API in one process) and `kira-cli` (local dashboard host that proxies to a remote `kira`). Deeper design notes live in `docs/architecture.md`.

## Setup

- Rust 1.88 (edition 2024) and Node/npm.
- `mprocs` and `cargo-watch` are needed for `make dev`.
- Run `npm ci` before `make check` or any OpenAPI/type generation; the Make target requires `node_modules`.
- Copy `.env.example` to `.env` when running `kira`; `DISCORD_TOKEN` is required only at runtime. A missing `.env` is tolerated and real environment variables take precedence.

## Commands

- `make check` — authoritative gate: fmt, clippy with `-D warnings`, workspace tests, OpenAPI drift, TypeScript type drift, typecheck, and dependency-boundary assertions. Run this before claiming work is complete.
- `make test` or `cargo test --workspace`.
- Focused test: `cargo test -p kira-server --test health`, or `cargo test -p kira-config <filter>`.
- `make dev` — start Vite, `kira`, and `kira-cli` together under `mprocs`.
- `cargo run -p kira` — bot + API on `127.0.0.1:8080`.
- `make build-cli` — builds the dashboard, then release `kira-cli`; run `target/release/kira-cli` for the local host on `:3000`.
- `npm run dev -w @kira/dashboard` — Vite dev server on `127.0.0.1:5173`; proxies `/api` and `/ws` to `kira-cli` on `127.0.0.1:3000`. A debug `kira-cli` proxies non-API requests to Vite instead of serving embedded assets.
- `make openapi` — regenerate `openapi/openapi.yaml` and `packages/api-types/src/schema.d.ts` after any route or DTO change.
- `cargo xtask openapi generate|check` — the alias is defined in `.cargo/config.toml`.

## Build gotchas

- Never use `cargo build --release --workspace`: it pulls dev-only `kira-api-docs`/`xtask` and frontend requirements into one build. Release targets name packages explicitly (`-p kira` or `-p kira-cli`).
- `cargo build --release -p kira-cli` fails unless `apps/dashboard/dist/index.html` exists and is a real build, not the debug placeholder. Use `make build-cli`, or `make dashboard` first. Debug builds auto-create a placeholder.
- `cargo build -p kira --no-default-features` is the bot-only build; `kira-server` must stay out of that dependency tree.
- Boundary rules enforced by `make check` via `cargo tree`: `kira-cli` must not depend on `kira-server`, `kira-bot`, `serenity`, or `kira-api-docs`; `kira` must not depend on `kira-api-docs` or `xtask`. Do not add those dependencies casually.

## Architecture and conventions

- Layers: leaves `kira-core`, `kira-config`, `kira-protocol`; libraries `kira-bot`, `kira-server`, `kira-client`; applications `kira`, `kira-cli`; dev-only `kira-api-docs` (capstone over `kira-server`) and `xtask`.
- Binaries use `anyhow::Result` with context at operational boundaries; reusable crates expose typed `thiserror` errors.
- OpenAPI ownership: `#[utoipa::path]` handlers in `kira-server`, `ToSchema` DTOs in `kira-protocol`, document assembly only in `kira-api-docs`. Never hand-edit generated contracts — run `make openapi`.
- Dashboard uses file-based TanStack Router (`src/routes/`, with committed `routeTree.gen.ts`) and a single `openapi-fetch` client in `src/data/fetcher.ts`. Add API domains by copying `src/data/__templates/` (excluded from `tsc` until copied). Components must consume hooks, not call `fetch` directly.
- Shared presentational components live in `packages/ui` (`@kira/ui`) and are generated with the shadcn CLI. Add components with `npx shadcn@latest add <name> -c apps/dashboard`; import them as `@kira/ui/components/<name>`. The dashboard stylesheet is `packages/ui/src/styles/globals.css`, imported once from `apps/dashboard/src/main.tsx`.
- Current code uses `println!`/`eprintln!`; there is no logging crate. Do not add one incidentally.
- Shutdown uses one `tokio::sync::watch` channel shared by bot and server. The bot deliberately avoids awaiting gateway startup when no shards exist, so bind failures can exit instead of hanging.

## Tests

- Binary integration tests spawn `env!("CARGO_BIN_EXE_kira")`, clear `DISCORD_TOKEN`/`KIRA_BIND`, and need no real Discord access.
- `crates/kira/tests/bind_failure.rs` uses dead HTTP(S) proxies and a 30-second timeout; it verifies a server bind failure exits rather than hangs.
- `.github/workflows/ci.yml` runs nightly `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`. A `changes` job gates those jobs with `dorny/paths-filter` and `.github/filters.yaml`; they run when `**/*.rs`, the workflow file, or the filter file changes. `make check` remains the full local and authoritative gate; it adds the OpenAPI/TypeScript drift checks, typecheck, and dependency-boundary assertions.
