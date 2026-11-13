# Configuration

Configuration is loaded from environment variables with `dotenvy` + `envy`.
Loading `.env` is best effort: a missing file is not an error, and real
environment variables are authoritative.

| Variable | Used by | Default | Required |
|---|---|---|---|
| `DISCORD_TOKEN` | `kira` | — | yes |
| `KIRA_GUILD_ID` | `kira` | — | yes |
| `KIRA_PASSWORD` | `kira` | generated PIN | no |
| `KIRA_BIND` | `kira` server | `127.0.0.1:8080` | no |
| `KIRA_ALLOWED_ORIGINS` | `kira` server | loopback dashboard/Vite origins | non-loopback binds |
| `KIRA_COOKIE_SECURE` | `kira` server | `false` | no |
| `KIRA_COOKIE_SAMESITE` | `kira` server | `lax` | no |
| `KIRA_CLI_BIND` | `kira-cli` | `127.0.0.1:3000` | no |

`.env` is gitignored. `.env.example` documents the variables without secrets.
Rotation of any previously exposed Discord token is an operational follow-up.

Notes:

- `KIRA_GUILD_ID` is required by server builds; the API serves only that guild.
- `KIRA_PASSWORD` is the dashboard PIN. When unset, a random six-digit PIN is
  generated at startup and printed to the log. Use a longer value when the
  server is reachable beyond the local machine.
- `KIRA_ALLOWED_ORIGINS` is a comma-separated list of exact origins such as
  `http://127.0.0.1:3000`. A non-loopback `KIRA_BIND` is refused without it.
  Loopback binds default to `127.0.0.1`/`localhost` on ports `3000` and `5173`.
- `KIRA_COOKIE_SECURE=true` marks the session cookie `Secure`; set it behind
  HTTPS. `KIRA_COOKIE_SAMESITE=none` requires `KIRA_COOKIE_SECURE=true`.
- `RUST_LOG` controls the `tracing` filter for both binaries (default `info`).

See [API.md](API.md) for route-level behavior and
[DEVELOPMENT.md](DEVELOPMENT.md) for local commands.
