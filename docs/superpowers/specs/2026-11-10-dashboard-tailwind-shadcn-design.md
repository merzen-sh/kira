# Dashboard Tailwind and shadcn UI package — design

**Date:** 2026-11-10
**Status:** proposed

## Intent

Give the Kira dashboard a real component system: Tailwind CSS for styling, shadcn
components generated from a registry, and a shared `@kira/ui` workspace so UI code
stops living inline in route files. The dashboard has no CSS today and no reusable
components; the only markup is the inline home page.

## Goals

- Tailwind CSS v4 wired into the dashboard build through `@tailwindcss/vite`.
- A `packages/ui` workspace named `@kira/ui` holding shadcn components, the `cn`
  utility, hooks, and the single global stylesheet.
- shadcn CLI configuration in both `packages/ui` and `apps/dashboard`, so future
  `add` commands route components into `packages/ui` and imports resolve.
- The dashboard imports components as `@kira/ui/components/...`; the home page uses
  generated components.
- One supported dependency direction: `apps/dashboard` → `packages/ui`; React is a
  peer dependency of the UI package.

## Non-goals

- No visual redesign beyond applying the generated components to the existing home
  page.
- No changes to data fetching, routing, API contracts, or Rust crates.
- No publishable package build (no bundler, no dist output); the workspace exports
  TypeScript source, matching `@kira/api-types`.
- No dark-mode toggle, theme editor, or component documentation site.

## Constraints

- npm workspaces (`apps/*`, `packages/*`); no pnpm/Turborepo.
- Vite 8, React 19, TypeScript 7, TanStack Router and Query.
- Tailwind v4 is CSS-first: no `tailwind.config.js`; theme tokens live in CSS.
- `apps/dashboard` builds with `tsc --noEmit && vite build`, and the Rust
  `kira-cli` serves that output.
- `make check` is the final gate; it includes TypeScript typecheck and the
  OpenAPI/API-types drift checks, which this change must not disturb.

## Approaches considered

1. **Source-only shared workspace with shadcn CLI (chosen).** `packages/ui` exports
   `.ts`/`.tsx` sources; Vite compiles them as part of the dashboard build. The
   shadcn CLI writes components into the package using `components.json` aliases.
   - Trade-off: no isolated build of the UI package; correctness is proven by the
     dashboard typecheck/build.
2. **Built component library.** Add `tsup`/Vite library mode and publish compiled
   output from `packages/ui`.
   - Rejected: adds a build pipeline and watch loop for no current consumer besides
     the dashboard.
3. **Components stay in `apps/dashboard/src/components/ui`.** Standard single-app
   shadcn setup.
   - Rejected: the request is explicitly to separate UI components into
     `packages/ui`.

## Decisions

- **shadcn base: Base UI** (`base-nova` style), the current shadcn default, chosen
  by the human partner.
- **Package name and layout:** `@kira/ui`, with `src/components`, `src/hooks`,
  `src/lib`, and `src/styles/globals.css`.
- **Exports:** `./globals.css`, `./components/*`, `./hooks/*`, `./lib/*`. Consumers
  import explicit paths; no barrel file.
- **Styling entry point:** `packages/ui/src/styles/globals.css` imports
  `tailwindcss`, the shadcn Tailwind styles, and `tw-animate-css`, and defines the
  theme tokens. The dashboard imports it once from `main.tsx`.
- **Tailwind source discovery:** explicit `@source` entries for the UI package and
  the dashboard source, because automatic detection does not scan linked workspace
  packages reliably.
- **Initial components:** `button`, `card`, and `badge`; the home page moves to
  `Card` + `Badge`, with a `Refresh` button calling the existing query's `refetch`.
- **CLI configuration:** `components.json` in both workspaces; `aliases.ui` in the
  dashboard points at `@kira/ui/components`, and `aliases.utils` at
  `@kira/ui/lib/utils`.
- **Type checking:** `packages/ui` gets its own `tsconfig.json` and `typecheck`
  script so the root `npm run typecheck --workspaces` covers it.
- **Theme:** shadcn `neutral` base color, CSS variables enabled, `lucide` icons.

## Left to the builder

- Exact shadcn CLI invocation (init versus hand-written configs plus `add`),
  choosing the path that avoids rewriting unrelated files.
- Whether `@kira/ui` dependencies are installed at the root or in the package, as
  long as the final `package.json`/lockfile reflect the real owners.
- Exact markup of the refactored home page; behavior stays the same.
- Any `@source` path adjustments needed for Tailwind to see every class.
