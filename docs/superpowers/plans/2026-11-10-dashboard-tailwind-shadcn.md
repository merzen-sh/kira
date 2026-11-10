# Dashboard Tailwind and shadcn UI Package Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Tailwind CSS v4 and shadcn Base UI components to the dashboard, with all shared UI code living in a new `@kira/ui` workspace at `packages/ui`.

**Architecture:** `packages/ui` is a source-only npm workspace exporting `.ts`/`.tsx` files and one global stylesheet. The dashboard consumes it through package exports, Vite compiles the sources, and Tailwind scans both workspaces through explicit `@source` entries. shadcn CLI configuration exists in both workspaces so components are written to `packages/ui` and imports resolve as `@kira/ui/...`.

**Tech Stack:** Tailwind CSS 4, `@tailwindcss/vite`, shadcn CLI (base-nova / Base UI), React 19, Vite 8, TypeScript 7, npm workspaces.

**Spec:** `docs/superpowers/specs/2026-11-10-dashboard-tailwind-shadcn-design.md`

## Global Constraints

- shadcn base is `base` (Base UI); `components.json` style is `base-nova`, `rsc: false`, `tsx: true`, `baseColor: "neutral"`, `cssVariables: true`, `iconLibrary: "lucide"`.
- Tailwind v4 is CSS-first: no `tailwind.config.js` or PostCSS config is added.
- `@kira/ui` exports only `./globals.css`, `./components/*`, `./hooks/*`, `./lib/*`; no barrel file.
- `react` and `react-dom` are peer dependencies of `@kira/ui`; they must not be regular dependencies.
- Dependency direction: `apps/dashboard` → `@kira/ui`. `@kira/ui` must not import anything from the dashboard.
- The Rust workspace and API contracts are untouched; `openapi/` and `packages/api-types/src/schema.d.ts` must not change.
- CI is path-filtered to `**/*.rs` and `.github/workflows/ci.yml`, so this branch's gate is local: `npm run build -w @kira/dashboard`, `npm run typecheck`, and `make check`.
- There is no frontend test runner today; adding one is out of scope. Verification is build, typecheck, and the built-CSS assertions named in each task.

## Review Focus

- Tailwind may not scan `packages/ui` through the workspace symlink, leaving every generated component unstyled. The built CSS must contain utilities used only by the new components (`bg-card`, `text-muted-foreground`).
- `@kira/ui` source exports may fail to resolve under the dashboard's `moduleResolution: "Bundler"`, breaking `tsc --noEmit`. Both workspace typechecks and the dashboard build must pass.
- React may be duplicated if `@kira/ui` declares it as a dependency instead of a peer. `npm ls react` must resolve to a single version.
- The shadcn CLI may write components into `apps/dashboard` instead of `packages/ui`. This is checked by file existence and the absence of `apps/dashboard/src/components/ui`.
- The theme may be missing when the dashboard skips the stylesheet import or imports it after render. The built CSS must contain `--primary:` and `--background:` tokens, and the home page must use them.
- The dashboard's own `tsc --noEmit` may not typecheck `packages/ui`; the root `npm run typecheck` must cover the UI package through its `typecheck` script.

---

### Task 1: Create `@kira/ui` and wire Tailwind into the dashboard

**Files:**
- Create: `packages/ui/package.json`
- Create: `packages/ui/tsconfig.json`
- Create: `packages/ui/components.json`
- Create: `packages/ui/src/lib/utils.ts`
- Create: `packages/ui/src/styles/globals.css`
- Create: `packages/ui/src/hooks/.gitkeep`
- Create: `apps/dashboard/components.json`
- Modify: `apps/dashboard/package.json`
- Modify: `apps/dashboard/vite.config.ts`
- Modify: `apps/dashboard/src/main.tsx`

**Interfaces:**
- Consumes: nothing.
- Produces: workspace `@kira/ui` with exports `./globals.css`, `./components/*`, `./hooks/*`, `./lib/*`; `src/lib/utils.ts` exporting `cn`; `src/styles/globals.css` with the neutral theme.

- [ ] **Step 1: Create the UI workspace manifest and TypeScript config**

`packages/ui/package.json`:

```json
{
  "name": "@kira/ui",
  "private": true,
  "type": "module",
  "exports": {
    "./globals.css": "./src/styles/globals.css",
    "./components/*": "./src/components/*.tsx",
    "./hooks/*": "./src/hooks/*.ts",
    "./lib/*": "./src/lib/*.ts"
  },
  "scripts": {
    "typecheck": "tsc --noEmit"
  },
  "peerDependencies": {
    "react": "^19.3.1",
    "react-dom": "^19.3.1"
  }
}
```

`packages/ui/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "paths": {
      "@kira/ui/*": ["./src/*"]
    },
    "jsx": "react-jsx",
    "strict": true,
    "noEmit": true,
    "skipLibCheck": true
  },
  "include": ["src"]
}
```

- [ ] **Step 2: Add the shadcn configuration to both workspaces**

`packages/ui/components.json`:

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "base-nova",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "",
    "css": "src/styles/globals.css",
    "baseColor": "neutral",
    "cssVariables": true
  },
  "iconLibrary": "lucide",
  "aliases": {
    "components": "@kira/ui/components",
    "ui": "@kira/ui/components",
    "lib": "@kira/ui/lib",
    "hooks": "@kira/ui/hooks",
    "utils": "@kira/ui/lib/utils"
  }
}
```

`apps/dashboard/components.json`:

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "base-nova",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "",
    "css": "../../packages/ui/src/styles/globals.css",
    "baseColor": "neutral",
    "cssVariables": true
  },
  "iconLibrary": "lucide",
  "aliases": {
    "components": "@/components",
    "ui": "@kira/ui/components",
    "lib": "@/lib",
    "hooks": "@/hooks",
    "utils": "@kira/ui/lib/utils"
  }
}
```

- [ ] **Step 3: Add the `cn` utility and the global stylesheet**

`packages/ui/src/lib/utils.ts`:

```ts
export { cn } from "cn";
```

`packages/ui/src/styles/globals.css` — use the default neutral theme exactly as published, with the two `@source` lines added after the imports:

```css
@import "tailwindcss";
@import "tw-animate-css";
@import "shadcn/tailwind.css";

@source "../../src";
@source "../../../../apps/dashboard/src";

@custom-variant dark (&:is(.dark *));

@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-card: var(--card);
  --color-card-foreground: var(--card-foreground);
  --color-popover: var(--popover);
  --color-popover-foreground: var(--popover-foreground);
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-secondary: var(--secondary);
  --color-secondary-foreground: var(--secondary-foreground);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-accent: var(--accent);
  --color-accent-foreground: var(--accent-foreground);
  --color-destructive: var(--destructive);
  --color-border: var(--border);
  --color-input: var(--input);
  --color-ring: var(--ring);
  --color-chart-1: var(--chart-1);
  --color-chart-2: var(--chart-2);
  --color-chart-3: var(--chart-3);
  --color-chart-4: var(--chart-4);
  --color-chart-5: var(--chart-5);
  --color-sidebar: var(--sidebar);
  --color-sidebar-foreground: var(--sidebar-foreground);
  --color-sidebar-primary: var(--sidebar-primary);
  --color-sidebar-primary-foreground: var(--sidebar-primary-foreground);
  --color-sidebar-accent: var(--sidebar-accent);
  --color-sidebar-accent-foreground: var(--sidebar-accent-foreground);
  --color-sidebar-border: var(--sidebar-border);
  --color-sidebar-ring: var(--sidebar-ring);
  --radius-sm: calc(var(--radius) * 0.6);
  --radius-md: calc(var(--radius) * 0.8);
  --radius-lg: var(--radius);
  --radius-xl: calc(var(--radius) * 1.4);
  --radius-2xl: calc(var(--radius) * 1.8);
  --radius-3xl: calc(var(--radius) * 2.2);
  --radius-4xl: calc(var(--radius) * 2.6);
}

:root {
  --radius: 0.625rem;
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
  --card: oklch(1 0 0);
  --card-foreground: oklch(0.145 0 0);
  --popover: oklch(1 0 0);
  --popover-foreground: oklch(0.145 0 0);
  --primary: oklch(0.205 0 0);
  --primary-foreground: oklch(0.985 0 0);
  --secondary: oklch(0.97 0 0);
  --secondary-foreground: oklch(0.205 0 0);
  --muted: oklch(0.97 0 0);
  --muted-foreground: oklch(0.556 0 0);
  --accent: oklch(0.97 0 0);
  --accent-foreground: oklch(0.205 0 0);
  --destructive: oklch(0.577 0.245 27.325);
  --border: oklch(0.922 0 0);
  --input: oklch(0.922 0 0);
  --ring: oklch(0.708 0 0);
  --chart-1: oklch(0.646 0.222 41.116);
  --chart-2: oklch(0.6 0.118 184.704);
  --chart-3: oklch(0.398 0.07 227.392);
  --chart-4: oklch(0.828 0.189 84.429);
  --chart-5: oklch(0.769 0.188 70.08);
  --sidebar: oklch(0.985 0 0);
  --sidebar-foreground: oklch(0.145 0 0);
  --sidebar-primary: oklch(0.205 0 0);
  --sidebar-primary-foreground: oklch(0.985 0 0);
  --sidebar-accent: oklch(0.97 0 0);
  --sidebar-accent-foreground: oklch(0.205 0 0);
  --sidebar-border: oklch(0.922 0 0);
  --sidebar-ring: oklch(0.708 0 0);
}

.dark {
  --background: oklch(0.145 0 0);
  --foreground: oklch(0.985 0 0);
  --card: oklch(0.205 0 0);
  --card-foreground: oklch(0.985 0 0);
  --popover: oklch(0.205 0 0);
  --popover-foreground: oklch(0.985 0 0);
  --primary: oklch(0.922 0 0);
  --primary-foreground: oklch(0.205 0 0);
  --secondary: oklch(0.269 0 0);
  --secondary-foreground: oklch(0.985 0 0);
  --muted: oklch(0.269 0 0);
  --muted-foreground: oklch(0.708 0 0);
  --accent: oklch(0.269 0 0);
  --accent-foreground: oklch(0.985 0 0);
  --destructive: oklch(0.704 0.191 22.216);
  --border: oklch(1 0 0 / 10%);
  --input: oklch(1 0 0 / 15%);
  --ring: oklch(0.556 0 0);
  --chart-1: oklch(0.488 0.243 264.376);
  --chart-2: oklch(0.696 0.17 162.48);
  --chart-3: oklch(0.769 0.188 70.08);
  --chart-4: oklch(0.627 0.265 303.9);
  --chart-5: oklch(0.645 0.246 16.439);
  --sidebar: oklch(0.205 0 0);
  --sidebar-foreground: oklch(0.985 0 0);
  --sidebar-primary: oklch(0.488 0.243 264.376);
  --sidebar-primary-foreground: oklch(0.985 0 0);
  --sidebar-accent: oklch(0.269 0 0);
  --sidebar-accent-foreground: oklch(0.985 0 0);
  --sidebar-border: oklch(1 0 0 / 10%);
  --sidebar-ring: oklch(0.556 0 0);
}

@layer base {
  * {
    @apply border-border outline-ring/50;
  }

  body {
    @apply bg-background text-foreground;
  }
}
```

Create `packages/ui/src/hooks/.gitkeep` so the exported `./hooks/*` path has a directory.

- [ ] **Step 4: Install the UI package dependencies**

Run from the repo root:

```bash
npm install -w @kira/ui @base-ui/react@^1.9.0 class-variance-authority@^0.7.1 cn@^0.5.2 lucide-react@^1.70.0
npm install -w @kira/ui -D shadcn@^4.23.0 tw-animate-css@^1.4.0 tailwindcss@^4.3.5 typescript@^7.0.2 @types/react@^19.3.0 @types/react-dom@^19.3.0
```

Expected: `packages/ui/package.json` gains `dependencies` and `devDependencies`; `package-lock.json` updates.

- [ ] **Step 5: Wire the dashboard to the UI package and Tailwind**

Add `"@kira/ui": "*"` to `apps/dashboard/package.json` `dependencies`, then run:

```bash
npm install
npm install -w @kira/dashboard -D @tailwindcss/vite@^4.3.5
```

In `apps/dashboard/vite.config.ts`, import `tailwindcss` from `@tailwindcss/vite` and add `tailwindcss()` to `plugins` after `react()`.

In `apps/dashboard/src/main.tsx`, make the first import:

```ts
import "@kira/ui/globals.css";
```

- [ ] **Step 6: Verify the theme reaches the dashboard build**

Run:

```bash
npm run typecheck
npm run build -w @kira/dashboard
grep -l -- "--primary:" apps/dashboard/dist/assets/*.css
grep -l -- "--background:" apps/dashboard/dist/assets/*.css
```

Expected: typecheck exits 0; the build writes `dist/index.html`; both `grep` commands print a CSS filename. This proves the stylesheet import and Tailwind compilation are wired.

- [ ] **Step 7: Commit**

```bash
git add packages/ui apps/dashboard/package.json apps/dashboard/components.json apps/dashboard/vite.config.ts apps/dashboard/src/main.tsx package-lock.json
git commit -m "feat(dashboard): add Tailwind and shared UI workspace"
```

---

### Task 2: Generate the shadcn components into `packages/ui`

**Files:**
- Create: `packages/ui/src/components/button.tsx`
- Create: `packages/ui/src/components/card.tsx`
- Create: `packages/ui/src/components/badge.tsx`
- Modify: `packages/ui/src/lib/utils.ts` (only if the CLI rewrites it)
- Modify: `packages/ui/package.json` (only if the CLI adds dependencies)

**Interfaces:**
- Consumes: Task 1's `@kira/ui` workspace, its `components.json` aliases, `cn` utility, and `globals.css`.
- Produces:
  - `@kira/ui/components/button`: `Button`, `buttonVariants`
  - `@kira/ui/components/card`: `Card`, `CardHeader`, `CardTitle`, `CardDescription`, `CardAction`, `CardContent`, `CardFooter`
  - `@kira/ui/components/badge`: `Badge`, `badgeVariants`

- [ ] **Step 1: Add the components with the shadcn CLI**

Run from the repo root, targeting the dashboard workspace so the CLI routes the
files through its `aliases.ui` (`@kira/ui/components`):

```bash
npx shadcn@latest add button card badge -c apps/dashboard -y
```

Expected: the CLI writes `packages/ui/src/components/{button,card,badge}.tsx` and leaves `apps/dashboard/src` untouched. It may install `cn`, `class-variance-authority`, and `@base-ui/react` and refresh `packages/ui/src/lib/utils.ts`.

If the CLI cannot resolve the shared workspace from the app, retry with
`-c packages/ui`. If it still misroutes or fails, fetch the registry items
(`https://ui.shadcn.com/r/styles/base-nova/<name>.json` for `button`, `card`,
and `badge`), write each `files[0].content` to `packages/ui/src/components/<name>.tsx`,
and install the `dependencies` lists from those items.

- [ ] **Step 2: Verify placement and the absence of app-local components**

Run:

```bash
test -f packages/ui/src/components/button.tsx
test -f packages/ui/src/components/card.tsx
test -f packages/ui/src/components/badge.tsx
test ! -e apps/dashboard/src/components/ui
```

Expected: all four commands exit 0. If the CLI wrote to `apps/dashboard/src/components/ui`, move the three files to `packages/ui/src/components/` and re-run the checks (record the deviation in the task notes).

- [ ] **Step 3: Verify the UI package typechecks**

Run:

```bash
npm run typecheck -w @kira/ui
npm ls react
```

Expected: typecheck exits 0; `npm ls react` shows one `react@19.3.1` (deduped), with `@kira/ui` listing it as a peer dependency, not a nested copy.

- [ ] **Step 4: Commit**

```bash
git add packages/ui package-lock.json
git commit -m "feat(ui): add shadcn button, card, and badge components"
```

---

### Task 3: Use the shared components on the home page

**Files:**
- Modify: `apps/dashboard/src/routes/index.tsx`

**Interfaces:**
- Consumes: `Button` from `@kira/ui/components/button`; `Card`, `CardHeader`, `CardTitle`, `CardDescription`, `CardContent`, `CardFooter` from `@kira/ui/components/card`; `Badge` from `@kira/ui/components/badge`; `useHealthQuery` from `@/data/health/query`.
- Produces: the home page's rendered `Card` layout, with a refresh action.

- [ ] **Step 1: Replace the inline markup with shared components**

`apps/dashboard/src/routes/index.tsx` becomes:

```tsx
import { Badge } from "@kira/ui/components/badge";
import { Button } from "@kira/ui/components/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";
import { createFileRoute } from "@tanstack/react-router";

import { useHealthQuery } from "@/data/health/query";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  const { data, isFetching, refetch } = useHealthQuery();
  const status = data?.status ?? "offline";
  const isHealthy = status === "ok";

  return (
    <main className="flex min-h-svh items-center justify-center bg-background p-6 text-foreground">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>Kira Dashboard</CardTitle>
          <CardDescription>Server health at a glance.</CardDescription>
        </CardHeader>
        <CardContent className="flex items-center gap-2">
          <span className="text-muted-foreground">Server</span>
          <Badge variant={isHealthy ? "default" : "destructive"}>
            {status}
          </Badge>
        </CardContent>
        <CardFooter>
          <Button
            variant="outline"
            disabled={isFetching}
            onClick={() => void refetch()}
          >
            {isFetching ? "Refreshing…" : "Refresh"}
          </Button>
        </CardFooter>
      </Card>
    </main>
  );
}
```

- [ ] **Step 2: Verify the dashboard typecheck and build**

Run:

```bash
npm run typecheck
npm run build -w @kira/dashboard
```

Expected: both exit 0.

- [ ] **Step 3: Verify Tailwind scanned both workspaces**

Run:

```bash
grep -l "bg-card" apps/dashboard/dist/assets/*.css
grep -l "text-muted-foreground" apps/dashboard/dist/assets/*.css
```

Expected: both commands print a CSS filename. If either fails, add the missing workspace directory to `packages/ui/src/styles/globals.css` as an `@source` line and rebuild; repeat until both pass.

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/index.tsx
git commit -m "feat(dashboard): use shared UI components on the home page"
```

---

### Task 4: Document the UI package and run the full gate

**Files:**
- Modify: `AGENTS.md`

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces: updated contributor guidance; green full gate.

- [ ] **Step 1: Document where UI components live**

In `AGENTS.md`, extend the dashboard bullet under "Architecture and conventions" with:

```markdown
- Shared presentational components live in `packages/ui` (`@kira/ui`) and are generated with the shadcn CLI. Add components with `npx shadcn@latest add <name> -c packages/ui`; import them as `@kira/ui/components/<name>`. The dashboard stylesheet is `packages/ui/src/styles/globals.css`, imported once from `apps/dashboard/src/main.tsx`.
```

- [ ] **Step 2: Run the full local gate**

Run:

```bash
make check
npm run build -w @kira/dashboard
```

Expected: `make check` exits 0 and the dashboard build exits 0. `make check` covers workspace typecheck, which now includes `@kira/ui`.

- [ ] **Step 3: Confirm no Rust or contract drift**

Run:

```bash
git status --short
git diff --stat main -- crates xtask openapi packages/api-types
```

Expected: the status contains only intended frontend/doc changes; the diff stat is empty for the Rust crates, `xtask`, `openapi`, and `packages/api-types`.

- [ ] **Step 4: Commit**

```bash
git add AGENTS.md
git commit -m "docs: describe the shared dashboard UI package"
```
