import { cn } from "@kira/ui/lib/utils";
import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import {
  LayoutDashboard,
  LogOut,
  Megaphone,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  Server,
  Settings,
  Sun,
} from "lucide-react";
import { useEffect, useState } from "react";

import { useLogoutMutation } from "@/data/auth/mutation";
import { useGuildQuery } from "@/data/guild/query";
import { useTheme } from "@/hooks/use-theme";

const NAV_ITEMS = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard },
  { to: "/announcements", label: "Announcements", icon: Megaphone },
  { to: "/server", label: "Server", icon: Server },
  { to: "/settings", label: "Settings", icon: Settings },
] as const;

const MOBILE_QUERY = "(max-width: 767px)";

export function DashboardShell() {
  const [collapsed, setCollapsed] = useState(
    () => typeof window !== "undefined" && window.matchMedia(MOBILE_QUERY).matches,
  );
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const logout = useLogoutMutation();

  useEffect(() => {
    const media = window.matchMedia(MOBILE_QUERY);
    const onChange = (event: MediaQueryListEvent) => {
      if (event.matches) {
        setCollapsed(true);
      }
    };

    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);

  return (
    <div className="flex min-h-svh bg-background text-foreground">
      {!collapsed ? (
        <button
          type="button"
          aria-label="Close navigation"
          className="fixed inset-y-0 right-0 left-14 z-30 bg-black/40 backdrop-blur-[1px] md:hidden"
          onClick={() => setCollapsed(true)}
        />
      ) : null}

      <aside
        id="dashboard-sidebar"
        aria-label="Primary navigation"
        className={cn(
          "sticky top-0 z-40 flex h-svh w-14 shrink-0 flex-col overflow-hidden border-r border-border bg-card transition-[width] duration-300 ease-in-out motion-reduce:transition-none",
          collapsed ? "w-14" : "w-60",
          "max-md:fixed max-md:inset-y-0 max-md:left-0",
          !collapsed && "max-md:shadow-2xl",
        )}
      >
        <button
          type="button"
          onClick={() => setCollapsed((value) => !value)}
          aria-expanded={!collapsed}
          aria-controls="dashboard-sidebar"
          title={collapsed ? "Expand navigation" : "Collapse navigation"}
          className="flex h-14 w-full shrink-0 items-center border-b border-border/60 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none"
        >
          <span className="flex h-14 w-14 shrink-0 items-center justify-center">
            {collapsed ? (
              <PanelLeftOpen className="size-5" />
            ) : (
              <PanelLeftClose className="size-5" />
            )}
          </span>
          <span
            className={cn(
              "min-w-0 truncate text-xs font-semibold tracking-[0.18em] uppercase transition-opacity duration-200",
              collapsed ? "opacity-0" : "pr-4 opacity-100",
            )}
          >
            Collapse
          </span>
        </button>

        <nav aria-label="Dashboard sections" className="flex flex-1 flex-col gap-1 py-2">
          {NAV_ITEMS.map(({ to, label, icon: Icon }) => {
            const active = pathname === to;

            return (
              <Link
                key={to}
                to={to}
                title={label}
                aria-current={active ? "page" : undefined}
                className={cn(
                  "flex h-10 items-center text-sm font-medium transition-colors focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none",
                  active
                    ? "bg-primary/10 text-primary"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                <span className="flex h-10 w-14 shrink-0 items-center justify-center">
                  <Icon className="size-5" />
                </span>
                <span
                  className={cn(
                    "min-w-0 truncate transition-opacity duration-200",
                    collapsed ? "opacity-0" : "pr-4 opacity-100",
                  )}
                >
                  {label}
                </span>
              </Link>
            );
          })}
        </nav>

        <div className="border-t border-border/60 p-2">
          <button
            type="button"
            onClick={() => logout.mutate()}
            disabled={logout.isPending}
            title="Sign out"
            aria-label="Sign out"
            className="flex h-10 w-full items-center text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none disabled:opacity-50"
          >
            <span className="flex h-10 w-14 shrink-0 items-center justify-center">
              <LogOut className="size-5" />
            </span>
            <span
              className={cn(
                "min-w-0 truncate transition-opacity duration-200",
                collapsed ? "opacity-0" : "pr-4 opacity-100",
              )}
            >
              Sign out
            </span>
          </button>
        </div>
      </aside>

      <div aria-hidden="true" className="w-14 shrink-0 md:hidden" />

      <div className="flex min-w-0 flex-1 flex-col">
        <TopBar />
        <div className="flex min-h-0 flex-1 flex-col">
          <Outlet />
        </div>
      </div>
    </div>
  );
}

function TopBar() {
  return (
    <header className="sticky top-0 z-20 flex h-14 shrink-0 items-center border-b border-border bg-card/95 px-3 backdrop-blur supports-[backdrop-filter]:bg-card/80">
      <Link
        to="/"
        aria-label="Kira dashboard"
        className="flex h-9 shrink-0 items-center gap-2 rounded-md pr-2 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none"
      >
        <span
          aria-hidden="true"
          className="flex size-8 items-center justify-center rounded-lg bg-gradient-to-br from-primary to-primary/60 text-sm font-semibold text-primary-foreground shadow-sm"
        >
          K
        </span>
        <span className="text-sm font-semibold tracking-tight">Kira</span>
      </Link>

      <div className="flex min-w-0 flex-1 items-center justify-center px-3">
        <GuildBadge />
      </div>

      <div className="flex shrink-0 items-center">
        <ThemeToggle />
      </div>
    </header>
  );
}

function GuildBadge() {
  const { data: guild, isError } = useGuildQuery();

  if (!guild) {
    return (
      <span className="truncate text-xs text-muted-foreground">
        {isError ? "Server unavailable" : "Loading server…"}
      </span>
    );
  }

  return (
    <Link
      to="/server"
      title={guild.name}
      className="flex min-w-0 items-center gap-2 rounded-md px-2 py-1 transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none"
    >
      {guild.icon_url ? (
        <img
          src={guild.icon_url}
          alt=""
          className="size-7 shrink-0 rounded-full object-cover ring-1 ring-border"
        />
      ) : (
        <span className="flex size-7 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
          <Server className="size-4" />
        </span>
      )}
      <span className="min-w-0 truncate text-sm font-medium">{guild.name}</span>
    </Link>
  );
}

function ThemeToggle() {
  const { theme, toggle } = useTheme();
  const next = theme === "dark" ? "light" : "dark";

  return (
    <button
      type="button"
      onClick={toggle}
      title={`Switch to ${next} mode`}
      aria-label={`Switch to ${next} mode`}
      className="flex size-9 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none"
    >
      {theme === "dark" ? <Sun className="size-4" /> : <Moon className="size-4" />}
    </button>
  );
}
