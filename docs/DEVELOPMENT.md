# Development

See [CONFIGURATION.md](CONFIGURATION.md) for environment variables and
[ARCHITECTURE.md](ARCHITECTURE.md) for crate layout.

## Make targets

| Target | Command / purpose |
|---|---|
| `make build` | `cargo build -p kira` |
| `make release` | `cargo build --release -p kira` |
| `make dev` | `mprocs` for Vite, `kira`, and `kira-cli` (requires `mprocs` and `cargo-watch`); stops all three on exit or Ctrl-C |
| `make install` | `bun install --frozen-lockfile` (via the `node_modules` prerequisite) |
| `make install-hooks` | Point Git at `.githooks`; checks formatting and clippy on commit |
| `make uninstall-hooks` | Remove the Git hooks path override |
| `make dashboard` | Build the Vite dashboard into `dist/` |
| `make build-cli` | Build dashboard, then release `kira-cli` without the dev proxy |
| `make build-all` | Build dashboard, then both release binaries |
| `make openapi` | Generate YAML and TypeScript types |
| `make test` | `cargo test --workspace` |
| `make check` | Full gate (see below) |
| `make fmt` / `make clippy` | Formatting / lints |
| `make clean` | Remove Cargo output, `node_modules`, and dashboard `dist` |

`make` with no target is equivalent to `make build`. The Makefile uses the
overridable variables `CARGO`, `BUN`, and `MPROCS` (defaults: `cargo`, `bun`,
`mprocs`), so a tool can be selected per invocation, for example
`make dashboard BUN=/opt/bun/bin/bun`.

`make dev` runs Vite, `kira`, and `kira-cli` under `mprocs` and traps `EXIT`,
`INT`, and `TERM` to kill the whole process group. When the command exits or you
press Ctrl-C, none of the three processes are left behind.

`make install-hooks` opts into the pre-commit hook. It runs
`cargo fmt --all -- --check` and both clippy passes from `make check`. The hook
is read-only: it never formats, stages, or stashes files, so staged, unstaged,
partially staged, and untracked changes are left exactly as they are.

`make check` runs:

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo clippy -p kira --no-default-features -- -D warnings`
4. `cargo test --workspace`
5. `cargo xtask openapi check`
6. `bun run check:api-types`
7. `bun run typecheck`
8. `bun run --filter @kira/dashboard test`
9. `cargo tree` boundary assertions:
   - `kira-cli` has no `kira-server`, `kira-bot`, or `serenity`
   - `kira --all-features` has no `kira-api-docs` or `xtask`

## CI

`.github/workflows/ci.yml` runs a `changes` detection job and four
verification jobs, gated by `dorny/paths-filter` and `.github/filters.yaml`:

| Job | Runs |
|---|---|
| Format | `cargo fmt --all --check` on the pinned Rust 1.88 toolchain |
| Clippy | workspace clippy with `-D warnings`, plus `kira --no-default-features` |
| Test | `cargo test --workspace`, the release `docs_release` test, and `cargo xtask openapi check` |
| Dashboard test | `bun run --filter @kira/dashboard test`, `bun run check:api-types`, and a dashboard build |

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
bun install --frozen-lockfile
make install-hooks          # optional: check formatting and lints before commits
cp .env.example .env        # fill DISCORD_TOKEN and KIRA_GUILD_ID

# Run everything in development (Vite, kira, kira-cli); Ctrl-C stops all three
make dev

# Run the main service (bot + API on :8080)
cargo run -p kira

# Run the local dashboard host (assets on :3000)
make build-cli
target/release/kira-cli

# Frontend development (Vite on :5173; browser calls kira :8080 directly)
bun run --filter @kira/dashboard dev

# Contract update after changing a route
make openapi

# Full verification before merging
make check
```

When `KIRA_PASSWORD` is unset, `kira` prints a generated PIN at startup; enter
it in the dashboard login screen.

## Logging

Set `RUST_LOG` to change the filter for either binary, for example
`RUST_LOG=debug cargo run -p kira`. The default is `info`; logs go to stderr.
