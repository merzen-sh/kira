# Development

See [CONFIGURATION.md](CONFIGURATION.md) for environment variables and
[ARCHITECTURE.md](ARCHITECTURE.md) for crate layout.

## Make targets

| Target | Command / purpose |
|---|---|
| `make build` | `cargo build -p kira` |
| `make release` | `cargo build --release -p kira` |
| `make dev` | `mprocs` for Vite, `kira`, and `kira-cli` |
| `make install` | `npm ci` (via the `node_modules` prerequisite) |
| `make dashboard` | Build the Vite dashboard into `dist/` |
| `make build-cli` | Build dashboard, then release `kira-cli` |
| `make build-all` | Build dashboard, then both release binaries |
| `make openapi` | Generate YAML and TypeScript types |
| `make test` | `cargo test --workspace` |
| `make check` | Full gate (see below) |
| `make fmt` / `make clippy` | Formatting / lints |
| `make clean` | Remove Cargo output, `node_modules`, and dashboard `dist` |

`make check` runs:

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo clippy -p kira --no-default-features -- -D warnings`
4. `cargo test --workspace`
5. `cargo xtask openapi check`
6. `npm run check:api-types`
7. `npm run typecheck`
8. `npm run test -w @kira/dashboard`
9. `cargo tree` boundary assertions:
   - `kira-cli` has no `kira-server`, `kira-bot`, or `serenity`
   - `kira --all-features` has no `kira-api-docs` or `xtask`

## CI

`.github/workflows/ci.yml` runs four jobs, gated by `dorny/paths-filter` and
`.github/filters.yaml`:

| Job | Runs |
|---|---|
| Format | `cargo fmt --all --check` on the pinned Rust 1.88 toolchain |
| Clippy | workspace clippy with `-D warnings`, plus `kira --no-default-features` |
| Test | `cargo test --workspace`, the release `docs_release` test, and `cargo xtask openapi check` |
| Dashboard test | `npm test -w @kira/dashboard`, `npm run check:api-types`, and a dashboard build |

Rust jobs run when Rust sources, manifests, lockfiles, tool config, or the
workflow/filter files change. The dashboard job runs when dashboard or shared
frontend packages change.

Never ship from `cargo build --release --workspace`: it would package the
dev-only OpenAPI tooling and the CLI's frontend requirements into one build.
Release targets name packages explicitly.

## Build matrix

```bash
cargo build --release -p kira
cargo build --release -p kira --no-default-features
make build-cli
bash scripts/smoke-test.sh
```

Release builds use LTO, one codegen unit, symbol stripping, and `panic = abort`.
Development builds use line-table debug info with split debuginfo.

## Quick reference

```bash
# First-time setup
npm ci
cp .env.example .env        # fill DISCORD_TOKEN and KIRA_GUILD_ID

# Run everything in development (Vite, kira, kira-cli)
make dev

# Run the main service (bot + API on :8080)
cargo run -p kira

# Run the local dashboard host (API on :3000)
make build-cli
target/release/kira-cli

# Frontend development (Vite on :5173 proxies to kira-cli :3000)
npm run dev -w @kira/dashboard

# Contract update after changing a route
make openapi

# Full verification before merging
make check
```

## Logging

Set `RUST_LOG` to change the filter for either binary, for example
`RUST_LOG=debug cargo run -p kira`. The default is `info`; logs go to stderr.
