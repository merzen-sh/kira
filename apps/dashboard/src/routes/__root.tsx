import { TanStackDevtools } from "@tanstack/react-devtools";
import { ReactQueryDevtoolsPanel } from "@tanstack/react-query-devtools";
import { createRootRoute } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";

import { DashboardShell } from "@/components/dashboard-shell";
import { LoginScreen } from "@/components/login-screen";
import { useSessionQuery } from "@/data/auth/query";

export const Route = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  const session = useSessionQuery();

  return (
    <>
      {session.isPending ? (
        <LoadingScreen />
      ) : session.data ? (
        <DashboardShell />
      ) : (
        <LoginScreen />
      )}
      {import.meta.env.DEV ? (
        <TanStackDevtools
          plugins={[
            {
              name: "TanStack Query",
              render: <ReactQueryDevtoolsPanel />,
            },
            {
              name: "TanStack Router",
              render: <TanStackRouterDevtoolsPanel />,
            },
          ]}
        />
      ) : null}
    </>
  );
}

function LoadingScreen() {
  return (
    <main className="flex min-h-svh flex-1 items-center justify-center bg-background p-6 text-foreground">
      <p className="text-sm text-muted-foreground" role="status">
        Connecting to the Kira server…
      </p>
    </main>
  );
}
