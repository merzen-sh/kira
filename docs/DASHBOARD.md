# Dashboard

`apps/dashboard` is a Vite + React + TypeScript dashboard. It shows server
health, the configured guild, and an announcements composer. TanStack Router
owns navigation; the unified TanStack Devtools panel hosts the Router and Query
panels in development.

## Routing

Routing is file-based:

- `src/routes/__root.tsx` owns the root layout and development devtools.
- `src/routes/index.tsx`, `server.tsx`, `settings.tsx`, and `announcements.tsx`
  are the page routes.
- The Vite router plugin generates `src/routeTree.gen.ts`; `src/router.tsx`
  creates the router from that tree.

## Data layer

The API layer lives in `src/data/`:

- `fetcher.ts` owns the single `openapi-fetch` client, the base URL, bearer
  token injection, and `unwrap` error normalization. Its `paths` type comes
  from `@kira/api-types`.
- Each API domain owns `keys.ts` and `query.ts` (plus `mutation.ts` when
  needed). `health/` is the first domain.
- TanStack Query owns server state. Components consume exported query hooks;
  they do not call `fetch` directly.
- `__templates/` holds copyable conventions for new domains.

Shared presentational components live in `packages/ui` (`@kira/ui`) and are
generated with the shadcn CLI. The global stylesheet is
`packages/ui/src/styles/globals.css`, imported once from
`apps/dashboard/src/main.tsx`. Add components with
`npx shadcn@latest add <name> -c apps/dashboard` and import them as
`@kira/ui/components/<name>`.

## Development path

In development, Vite runs on its own port and proxies `/api` and `/ws` to
`kira-cli` on `127.0.0.1:3000`. A debug `kira-cli` also proxies non-API
requests back to Vite on `127.0.0.1:5173`, so either entrypoint works:

```
Browser → Vite dev server :5173 → kira-cli :3000 → remote kira :8080
```

In production, `kira-cli` serves the built assets itself.

## Build and embedding

- `npm run build -w @kira/dashboard` runs `tsc --noEmit` and Vite, producing
  `apps/dashboard/dist/` with `emptyOutDir: true`.
- `kira-cli` embeds that directory with `rust-embed`.
  - **Release:** files are embedded into the binary at compile time and served.
  - **Debug:** non-API HTTP requests are proxied to the Vite dev server on
    `127.0.0.1:5173`; the embedded set is not used while that proxy is active.
- If the dev server URL is unset in a debug build (as in tests), rust-embed
  reads the embedded files from disk at runtime, so rebuilding the dashboard
  does not require a Rust rebuild.
- `crates/kira-cli/build.rs` enforces the build order:
  - A release build fails with `dashboard assets missing; run make dashboard`
    when `index.html` is missing or the `.kira-placeholder` marker exists.
  - A debug build creates a placeholder `index.html` and the marker when the
    dashboard has not been built.
  - A real Vite build removes the marker because it empties the output
    directory.
- `apps/dashboard/dist/` is gitignored; the built assets are not committed.
- `npm test -w @kira/dashboard` runs Vitest in the dashboard workspace; test
  files live next to the code they cover as `*.test.ts`. `make check` runs it.

`cargo build --release -p kira-cli` is independent of the `kira` application,
but it requires `make dashboard` first. `make build-cli` performs both steps.

See [API.md](API.md) for the proxied routes and
[DEVELOPMENT.md](DEVELOPMENT.md) for commands.
