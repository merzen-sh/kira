# Kira documentation

Kira is a Rust workspace plus npm workspaces. It produces two binaries: `kira`
(Discord bot + HTTP/WebSocket API in one process) and `kira-cli` (local
dashboard host that proxies to a remote `kira`).

Start with the document that matches the task:

| Document | Covers |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System overview, repository layout, crate boundaries, runtime composition, design decisions, scope |
| [API.md](API.md) | HTTP and WebSocket surface, authentication, OpenAPI and TypeScript contract |
| [CONFIGURATION.md](CONFIGURATION.md) | Environment variables and `.env` behavior |
| [DASHBOARD.md](DASHBOARD.md) | React app structure, data layer, build and embedding |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Make targets, verification gate, build matrix, quick reference |

For a first contribution, read [ARCHITECTURE.md](ARCHITECTURE.md) and
[DEVELOPMENT.md](DEVELOPMENT.md), then the document for the area you touch.
