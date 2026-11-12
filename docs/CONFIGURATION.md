# Configuration

Configuration is loaded from environment variables with `dotenvy` + `envy`.
Loading `.env` is best effort: a missing file is not an error, and real
environment variables are authoritative.

| Variable | Used by | Default | Required |
|---|---|---|---|
| `DISCORD_TOKEN` | `kira` | — | yes |
| `KIRA_GUILD_ID` | `kira` | — | yes |
| `KIRA_API_TOKEN` | `kira`, `kira-cli` | — | no |
| `KIRA_BIND` | `kira` server | `127.0.0.1:8080` | no |
| `KIRA_CLI_BIND` | `kira-cli` | `127.0.0.1:3000` | no |
| `KIRA_REMOTE_URL` | `kira-cli` | `http://127.0.0.1:8080` | no |

`.env` is gitignored. `.env.example` documents the variables without secrets.
Rotation of any previously exposed Discord token is an operational follow-up.

Notes:

- `KIRA_GUILD_ID` is required by server builds; the API serves only that guild.
- `KIRA_API_TOKEN` protects the guild and message routes. Set the same value for
  `kira` and `kira-cli` so the proxy can authenticate. Any non-loopback
  `KIRA_BIND` is refused without it.
- `RUST_LOG` controls the `tracing` filter for both binaries (default `info`).

See [API.md](API.md) for route-level behavior.
