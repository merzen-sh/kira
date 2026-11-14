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
| `KIRA_ALLOWED_ORIGINS` | `kira` server | derived from `KIRA_CLI_BIND` | non-loopback binds |
| `KIRA_HTTPS` | `kira` server | `false` | non-loopback binds |
| `KIRA_COOKIE_SECURE` | `kira` server | `false` | non-loopback binds |
| `KIRA_COOKIE_SAMESITE` | `kira` server | `lax` | no |
| `KIRA_CLI_BIND` | `kira`, `kira-cli` | `127.0.0.1:3000` | no |
| `KIRA_DATABASE_URL` | `kira` | `sqlite:kira.db` | no |

`.env` is gitignored. `.env.example` documents the variables without secrets.
Rotation of any previously exposed Discord token is an operational follow-up.

Notes:

- `KIRA_GUILD_ID` is required by server builds; the API serves only that guild.
- `KIRA_PASSWORD` is the dashboard PIN. When unset, a random six-digit PIN is
  generated at startup and printed to the log. Use a longer value when the
  server is reachable beyond the local machine.
- `KIRA_ALLOWED_ORIGINS` is a comma-separated list of exact origins such as
  `http://127.0.0.1:3000`. Loopback deployments derive defaults from
  `KIRA_CLI_BIND` (both `127.0.0.1` and `localhost`) plus the Vite dev server on
  `5173`. Set it explicitly when the dashboard or server binds beyond loopback
  or uses another hostname.
- `KIRA_HTTPS=true` asserts that TLS terminates in front of `kira`. A
  non-loopback `KIRA_BIND` is refused unless `KIRA_HTTPS=true`,
  `KIRA_COOKIE_SECURE=true`, and `KIRA_ALLOWED_ORIGINS` are all set. Loopback
  HTTP remains supported.
- `KIRA_COOKIE_SECURE=true` marks the session cookie `Secure`; set it behind
  HTTPS. `KIRA_COOKIE_SAMESITE=none` requires `KIRA_COOKIE_SECURE=true`.
- `KIRA_DATABASE_URL` is the SQLite connection URL used for bug reports. The
  file is created when missing, and pending migrations in `migrations/` are
  applied at startup.
- `RUST_LOG` controls the `tracing` filter for both binaries (default `info`).

See [API.md](API.md) for route-level behavior and
[DEVELOPMENT.md](DEVELOPMENT.md) for local commands.
