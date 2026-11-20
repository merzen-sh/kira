# Configuration

Configuration is loaded from environment variables with `dotenvy` + `envy`.
Loading `.env` is best effort: a missing file is not an error, and real
environment variables take precedence over values in the file.

| Variable | Used by | Default | Required |
|---|---|---|---|
| `DISCORD_TOKEN` | `kira` | — | yes |
| `KIRA_GUILD_ID` | `kira` server builds | — | yes in server builds |
| `KIRA_PASSWORD` | `kira` | generated PIN | no |
| `KIRA_BIND` | `kira` server | `127.0.0.1:8080` | no |
| `KIRA_ALLOWED_ORIGINS` | `kira` server | derived for loopback from `KIRA_CLI_BIND` | non-loopback binds |
| `KIRA_HTTPS` | `kira` server | `false` | non-loopback `KIRA_BIND` |
| `KIRA_COOKIE_SECURE` | `kira` server | `false` | non-loopback `KIRA_BIND`; required with `SameSite=None` |
| `KIRA_COOKIE_SAMESITE` | `kira` server | `lax` | no |
| `KIRA_CLI_BIND` | `kira`, `kira-cli` | `127.0.0.1:3000` | no |
| `KIRA_DATABASE_URL` | `kira` | `sqlite:kira.db` | no |

`.env` is gitignored. `.env.example` documents the variables without secrets.

Notes:

- `DISCORD_TOKEN` is required to start the bot. `--version` and `--help` work
  before configuration is loaded.
- `KIRA_GUILD_ID` selects the one Discord server exposed by the API. It is
  required in server builds only; a bot-only build registers slash commands
  globally and does not need it.
- `KIRA_PASSWORD` is the dashboard PIN. When unset or empty, a random six-digit
  PIN is generated at startup and logged at `warn`. Use a longer value when the
  server is reachable beyond the local machine.
- `KIRA_BIND` is the API listen address. Loopback HTTP is supported. A
  non-loopback address is refused unless `KIRA_HTTPS=true`,
  `KIRA_COOKIE_SECURE=true`, and `KIRA_ALLOWED_ORIGINS` are all set.
- `KIRA_ALLOWED_ORIGINS` is a comma-separated list of exact origins such as
  `http://127.0.0.1:3000`. Loopback deployments derive defaults from
  `KIRA_CLI_BIND` (both `127.0.0.1` and `localhost`) plus the Vite dev server on
  port `5173`. Set it explicitly when the dashboard or server binds beyond
  loopback, uses another hostname, or is reached through a different origin.
- `KIRA_HTTPS=true` asserts that TLS terminates in front of `kira`. TLS is not
  served by the binary itself.
- `KIRA_COOKIE_SECURE=true` marks the session cookie `Secure`; set it behind
  HTTPS. `KIRA_COOKIE_SAMESITE` accepts `lax`, `strict`, or `none`, and `none`
  requires `KIRA_COOKIE_SECURE=true`.
- `KIRA_CLI_BIND` is the dashboard host address. `kira` also uses it to derive
  default allowed origins for loopback deployments. A non-loopback value
  requires an explicit `KIRA_ALLOWED_ORIGINS` list.
- `KIRA_DATABASE_URL` is the SQLite connection URL for all persisted state:
  reports, guild users and join/leave events, monitor samples, audit events, and
  bot presence. The file is created when missing, and pending migrations in
  `migrations/` are applied at startup.
- `RUST_LOG` controls the `tracing` filter for both binaries (default `info`).

See [API.md](API.md) for route-level behavior and
[DEVELOPMENT.md](DEVELOPMENT.md) for local commands.
