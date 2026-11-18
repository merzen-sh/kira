import { Badge } from "@kira/ui/components/badge";
import { Button } from "@kira/ui/components/button";
import { Checkbox } from "@kira/ui/components/checkbox";
import { Input } from "@kira/ui/components/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@kira/ui/components/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@kira/ui/components/table";
import { createFileRoute } from "@tanstack/react-router";
import { Bug, CircleAlert, CircleCheck, UserRound } from "lucide-react";
import { useEffect, useState } from "react";

import { StatCard } from "@/components/stat-card";
import {
  useDeleteReportMutation,
  useResolveReportMutation,
} from "@/data/reports/mutation";
import { useReportStatsQuery, useReportsQuery } from "@/data/reports/query";
import { formatDateTime } from "@/lib/format-date";

export const Route = createFileRoute("/reports")({
  component: ReportsPage,
});

const PAGE_LIMIT = 20;

const RESOLVED_ITEMS = {
  all: "All reports",
  open: "Open",
  resolved: "Resolved",
};

function ReportsPage() {
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState("");
  const [term, setTerm] = useState("");
  const [resolved, setResolved] = useState("all");
  const [seenLastPage, setSeenLastPage] = useState(0);
  const resolve = useResolveReportMutation();
  const remove = useDeleteReportMutation();

  useEffect(() => {
    const timer = setTimeout(() => {
      setTerm(search.trim());
      setPage(1);
    }, 300);

    return () => clearTimeout(timer);
  }, [search]);

  const { data, isPending, isError, error } = useReportsQuery(
    page,
    PAGE_LIMIT,
    term,
    resolved,
  );
  const stats = useReportStatsQuery();
  const reports = data?.reports ?? [];
  const total = data?.total ?? 0;
  const lastPage = Math.max(1, Math.ceil(total / PAGE_LIMIT));

  // Another admin may delete the last row while this page is open. Clamp an
  // over-shoot page during render (React re-renders immediately without
  // committing the stale frame) instead of reacting in an effect.
  if (data && lastPage !== seenLastPage) {
    setSeenLastPage(lastPage);
    if (page > lastPage) {
      setPage(lastPage);
    }
  }

  function handleDelete(id: number) {
    if (!window.confirm("Delete this report?")) {
      return;
    }

    remove.mutate(id, {
      onSuccess: () => {
        if (reports.length === 1 && page > 1) {
          setPage(page - 1);
        }
      },
    });
  }

  return (
    <main className="flex flex-1 flex-col gap-4 p-6 md:p-8">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">Reports</h1>
          <p className="text-sm text-muted-foreground">
            Bug reports submitted with the Discord /report command.
          </p>
        </div>
        <div className="flex w-full flex-wrap gap-2 sm:w-auto">
          <Select
            value={resolved}
            onValueChange={(value) => {
              setResolved(String(value ?? "all"));
              setPage(1);
            }}
            items={RESOLVED_ITEMS}
          >
            <SelectTrigger
              aria-label="Filter by resolved state"
              className="w-full sm:w-36"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All reports</SelectItem>
              <SelectItem value="open">Open</SelectItem>
              <SelectItem value="resolved">Resolved</SelectItem>
            </SelectContent>
          </Select>
          <Input
            type="search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search reports…"
            aria-label="Search reports"
            className="w-full sm:w-64"
          />
        </div>
      </header>

      <section
        aria-label="Report statistics"
        className="grid gap-3 sm:grid-cols-3"
      >
        <StatCard
          label="Total reports"
          icon={Bug}
          value={stats.data?.total}
          pending={stats.isPending}
          error={stats.isError}
        />
        <StatCard
          label="Open"
          icon={CircleAlert}
          value={stats.data?.open}
          pending={stats.isPending}
          error={stats.isError}
        />
        <StatCard
          label="Resolved"
          icon={CircleCheck}
          value={stats.data?.resolved}
          pending={stats.isPending}
          error={stats.isError}
        />
      </section>

      {isError ? (
        <p role="alert" className="text-sm text-destructive">
          {error.message}
        </p>
      ) : null}

      {resolve.isError ? (
        <p role="alert" className="text-sm text-destructive">
          {resolve.error.message}
        </p>
      ) : null}

      {remove.isError ? (
        <p role="alert" className="text-sm text-destructive">
          {remove.error.message}
        </p>
      ) : null}

      <div className="rounded-xl border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-10">Done</TableHead>
              <TableHead>Title</TableHead>
              <TableHead>Steps</TableHead>
              <TableHead>Details</TableHead>
              <TableHead>Reporter</TableHead>
              <TableHead>Submitted</TableHead>
              <TableHead className="w-10" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {isPending ? (
              <TableRow>
                <TableCell colSpan={7} className="text-muted-foreground">
                  Loading reports…
                </TableCell>
              </TableRow>
            ) : reports.length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} className="text-muted-foreground">
                  {term || resolved !== "all"
                    ? "No reports match these filters."
                    : "No reports yet."}
                </TableCell>
              </TableRow>
            ) : (
              reports.map((report) => (
                <TableRow key={report.id}>
                  <TableCell>
                    <Checkbox
                      checked={report.resolved}
                      disabled={resolve.isPending}
                      onCheckedChange={(checked) =>
                        resolve.mutate({
                          id: report.id,
                          resolved: checked === true,
                        })
                      }
                      aria-label={`Mark report ${report.id} as ${
                        report.resolved ? "unresolved" : "resolved"
                      }`}
                    />
                  </TableCell>
                  <TableCell className="font-medium whitespace-normal">
                    {report.title}
                    {report.resolved ? (
                      <Badge variant="secondary" className="ml-2">
                        Resolved
                      </Badge>
                    ) : null}
                  </TableCell>
                  <TableCell className="max-w-64 whitespace-normal text-muted-foreground">
                    {report.steps}
                  </TableCell>
                  <TableCell className="max-w-64 whitespace-normal text-muted-foreground">
                    {report.details}
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      {report.avatar_url ? (
                        <img
                          src={report.avatar_url}
                          alt=""
                          className="size-7 shrink-0 rounded-full object-cover ring-1 ring-border"
                        />
                      ) : (
                        <span className="flex size-7 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
                          <UserRound className="size-4" />
                        </span>
                      )}
                      <div className="min-w-0">
                        <div className="truncate text-sm font-medium">
                          {report.display_name ??
                            report.user_name ??
                            report.user_id}
                        </div>
                        {report.user_name ? (
                          <div className="truncate font-mono text-xs text-muted-foreground">
                            {report.display_name
                              ? `@${report.user_name} · ${report.user_id}`
                              : report.user_id}
                          </div>
                        ) : null}
                      </div>
                    </div>
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {formatDateTime(report.created_at)}
                  </TableCell>
                  <TableCell>
                    <Button
                      variant="destructive"
                      size="sm"
                      disabled={remove.isPending}
                      onClick={() => handleDelete(report.id)}
                    >
                      Delete
                    </Button>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      <footer className="flex items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">
          {total} {total === 1 ? "report" : "reports"}
        </p>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={page <= 1}
            onClick={() => setPage(page - 1)}
          >
            Previous
          </Button>
          <span className="text-sm text-muted-foreground">
            Page {page} of {lastPage}
          </span>
          <Button
            variant="outline"
            size="sm"
            disabled={page >= lastPage}
            onClick={() => setPage(page + 1)}
          >
            Next
          </Button>
        </div>
      </footer>
    </main>
  );
}
