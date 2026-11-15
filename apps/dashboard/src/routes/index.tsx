import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";
import { Link, createFileRoute } from "@tanstack/react-router";
import { Bug, Users } from "lucide-react";

import { useReportStatsQuery } from "@/data/reports/query";
import { useUserStatsQuery } from "@/data/users/query";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function statValue(value: number | undefined, pending: boolean, error: boolean) {
  return pending ? "…" : error ? "—" : (value ?? 0);
}

function HomePage() {
  const reports = useReportStatsQuery();
  const users = useUserStatsQuery();

  return (
    <main className="flex flex-1 flex-col gap-4 p-6 md:p-8">
      <header>
        <h1 className="text-xl font-semibold">Dashboard</h1>
        <p className="text-sm text-muted-foreground">
          Reports and guild members at a glance.
        </p>
      </header>

      <section
        aria-label="Dashboard statistics"
        className="grid gap-3 sm:grid-cols-2"
      >
        <Link
          to="/reports"
          className="rounded-xl focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
        >
          <Card className="h-full transition-colors hover:bg-muted/50">
            <CardHeader>
              <CardDescription className="flex items-center gap-2">
                <Bug className="size-4" />
                Reports
              </CardDescription>
              <CardTitle className="text-2xl font-semibold tabular-nums">
                {statValue(
                  reports.data?.total,
                  reports.isPending,
                  reports.isError,
                )}
              </CardTitle>
            </CardHeader>
          </Card>
        </Link>
        <Link
          to="/users"
          className="rounded-xl focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
        >
          <Card className="h-full transition-colors hover:bg-muted/50">
            <CardHeader>
              <CardDescription className="flex items-center gap-2">
                <Users className="size-4" />
                Users
              </CardDescription>
              <CardTitle className="text-2xl font-semibold tabular-nums">
                {statValue(
                  users.data?.total,
                  users.isPending,
                  users.isError,
                )}
              </CardTitle>
            </CardHeader>
          </Card>
        </Link>
      </section>
    </main>
  );
}
