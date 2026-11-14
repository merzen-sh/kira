import { Badge } from "@kira/ui/components/badge";
import { Button } from "@kira/ui/components/button";
import { Checkbox } from "@kira/ui/components/checkbox";
import { Input } from "@kira/ui/components/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@kira/ui/components/table";
import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import {
  useDeleteReportMutation,
  useResolveReportMutation,
} from "@/data/reports/mutation";
import { useReportsQuery } from "@/data/reports/query";

export const Route = createFileRoute("/reports")({
  component: ReportsPage,
});

const PAGE_LIMIT = 20;

function ReportsPage() {
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState("");
  const [term, setTerm] = useState("");
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
  );
  const reports = data?.reports ?? [];
  const total = data?.total ?? 0;
  const lastPage = Math.max(1, Math.ceil(total / PAGE_LIMIT));

  // Another admin may delete the last row while this page is open.
  useEffect(() => {
    if (data && page > lastPage) {
      setPage(lastPage);
    }
  }, [data, lastPage, page]);

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
        <Input
          type="search"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          placeholder="Search reports…"
          aria-label="Search reports"
          className="w-full sm:w-64"
        />
      </header>

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
                  {term ? "No reports match this search." : "No reports yet."}
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
                  <TableCell className="font-mono text-xs">
                    {report.user_id}
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {new Date(report.created_at).toLocaleString()}
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
