# Kira documentation

Kira is a Rust workspace plus Bun workspaces. It builds two binaries:

- **`kira`** — the Discord bot and the HTTP/WebSocket API in one process.
- **`kira-cli`** — a local host for the built dashboard. Release builds serve
  embedded assets; debug builds proxy non-API requests to Vite. The browser
  always talks to `kira` directly for API and WebSocket traffic.

Start with the document that matches the task:

| Document | Covers |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System overview, repository layout, crate graph and boundaries, runtime composition, design decisions, scope |
| [API.md](API.md) | HTTP and WebSocket surface, authentication, origins, OpenAPI and TypeScript contract |
| [CONFIGURATION.md](CONFIGURATION.md) | Environment variables, defaults, `.env` behavior, deployment constraints |
| [DASHBOARD.md](DASHBOARD.md) | React app structure, routing, data layer, build and embedding |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Make targets, verification gate, CI, build matrix, quick reference |

For a first contribution, read [ARCHITECTURE.md](ARCHITECTURE.md) and
[DEVELOPMENT.md](DEVELOPMENT.md), then the document for the area you touch.
`AGENTS.md` at the repository root summarizes setup and boundary rules for
automated contributors.

All commands assume Rust 1.88 and Bun.
