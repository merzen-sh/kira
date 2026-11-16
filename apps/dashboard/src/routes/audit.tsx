import { Badge } from "@kira/ui/components/badge";
import { Button } from "@kira/ui/components/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";
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
import { useEffect, useState } from "react";

import {
  type AuditEvent,
  useAuditEventsQuery,
  useAuditFacetsQuery,
} from "@/data/audit/query";

export const Route = createFileRoute("/audit")({
  component: AuditPage,
});

const PAGE_LIMIT = 20;

function formatDate(value: string) {
  return new Date(value).toLocaleString();
}

function entityText(entity: AuditEvent["actor"] | null | undefined) {
  if (!entity) {
    return "—";
  }

  return [entity.name, entity.id, `(${entity.kind})`]
    .filter(Boolean)
    .join(" · ");
}

function Entity({ entity }: { entity: AuditEvent["actor"] | null | undefined }) {
  if (!entity) {
    return <span className="text-muted-foreground">—</span>;
  }

  const label = entity.name ?? entity.id ?? entity.kind;
  const detail =
    entity.name && entity.id
      ? `${entity.kind} · ${entity.id}`
      : entity.name || entity.id
        ? entity.kind
        : null;

  return (
    <span className="flex min-w-0 flex-col">
      <span className="truncate">{label}</span>
      {detail ? (
        <span className="truncate text-xs text-muted-foreground">{detail}</span>
      ) : null}
    </span>
  );
}

function AuditPage() {
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState("");
  const [term, setTerm] = useState("");
  const [action, setAction] = useState("");
  const [actorKind, setActorKind] = useState("");
  const [source, setSource] = useState("");
  const [selected, setSelected] = useState<AuditEvent | null>(null);

  useEffect(() => {
    const timer = setTimeout(() => {
      setTerm(search.trim());
      setPage(1);
    }, 300);

    return () => clearTimeout(timer);
  }, [search]);

  useEffect(() => {
    setSelected(null);
  }, [page, term, action, actorKind, source]);

  const facets = useAuditFacetsQuery();
  const { data, isPending, isError, error } = useAuditEventsQuery({
    page,
    pageLimit: PAGE_LIMIT,
    search: term,
    action,
    actorKind,
    source,
  });
  const events = data?.events ?? [];
  const total = data?.total ?? 0;
  const lastPage = Math.max(1, Math.ceil(total / PAGE_LIMIT));
  const filtered = Boolean(term || action || actorKind || source);

  const actionItems = {
    all: "All actions",
    ...Object.fromEntries(
      (facets.data?.actions ?? []).map((value) => [value, value]),
    ),
  };
  const actorItems = {
    all: "All actors",
    ...Object.fromEntries(
      (facets.data?.actor_kinds ?? []).map((value) => [value, value]),
    ),
  };
  const sourceItems = {
    all: "All sources",
    ...Object.fromEntries(
      (facets.data?.sources ?? []).map((value) => [value, value]),
    ),
  };

  return (
    <main className="flex flex-1 flex-col gap-4 p-6 md:p-8">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">Audit log</h1>
          <p className="text-sm text-muted-foreground">
            Generic, append-only event history for administrators.
          </p>
        </div>
        <div className="flex w-full flex-wrap gap-2 sm:w-auto">
          <Select
            value={action || "all"}
            onValueChange={(value) => {
              setAction(value === "all" ? "" : String(value ?? ""));
              setPage(1);
            }}
            items={actionItems}
          >
            <SelectTrigger
              aria-label="Filter by action"
              className="w-full sm:w-44"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All actions</SelectItem>
              {(facets.data?.actions ?? []).map((value) => (
                <SelectItem key={value} value={value}>
                  {value}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select
            value={actorKind || "all"}
            onValueChange={(value) => {
              setActorKind(value === "all" ? "" : String(value ?? ""));
              setPage(1);
            }}
            items={actorItems}
          >
            <SelectTrigger
              aria-label="Filter by actor kind"
              className="w-full sm:w-40"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All actors</SelectItem>
              {(facets.data?.actor_kinds ?? []).map((value) => (
                <SelectItem key={value} value={value}>
                  {value}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select
            value={source || "all"}
            onValueChange={(value) => {
              setSource(value === "all" ? "" : String(value ?? ""));
              setPage(1);
            }}
            items={sourceItems}
          >
            <SelectTrigger
              aria-label="Filter by source"
              className="w-full sm:w-36"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All sources</SelectItem>
              {(facets.data?.sources ?? []).map((value) => (
                <SelectItem key={value} value={value}>
                  {value}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            type="search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search audit events…"
            aria-label="Search audit events"
            className="w-full sm:w-64"
          />
        </div>
      </header>

      {isError ? (
        <p role="alert" className="text-sm text-destructive">
          {error.message}
        </p>
      ) : null}
      {facets.isError ? (
        <p role="alert" className="text-sm text-destructive">
          Audit filters unavailable: {facets.error.message}
        </p>
      ) : null}

      <div className="rounded-xl border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Occurred</TableHead>
              <TableHead>Action</TableHead>
              <TableHead>Source</TableHead>
              <TableHead>Actor</TableHead>
              <TableHead>Target</TableHead>
              <TableHead className="text-right">Details</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {isPending ? (
              <TableRow>
                <TableCell colSpan={6} className="text-muted-foreground">
                  Loading audit events…
                </TableCell>
              </TableRow>
            ) : events.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-muted-foreground">
                  {filtered
                    ? "No audit events match these filters."
                    : "No audit events yet."}
                </TableCell>
              </TableRow>
            ) : (
              events.map((event) => (
                <TableRow key={event.id}>
                  <TableCell className="whitespace-nowrap text-muted-foreground">
                    {formatDate(event.occurred_at)}
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary">{event.action}</Badge>
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline">{event.source}</Badge>
                  </TableCell>
                  <TableCell>
                    <Entity entity={event.actor} />
                  </TableCell>
                  <TableCell>
                    <Entity entity={event.target} />
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="outline"
                      size="sm"
                      aria-label={`Inspect event ${event.id} (${event.action})`}
                      onClick={() => setSelected(event)}
                    >
                      Inspect
                    </Button>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {selected ? (
        <Card
          size="sm"
          role="region"
          aria-live="polite"
          aria-labelledby="audit-event-detail"
        >
          <CardHeader>
            <CardDescription>Event #{selected.id}</CardDescription>
            <CardTitle id="audit-event-detail">{selected.action}</CardTitle>
            <CardAction>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setSelected(null)}
              >
                Close
              </Button>
            </CardAction>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <dl className="grid gap-3 text-sm sm:grid-cols-2">
              <div>
                <dt className="text-xs text-muted-foreground">Source</dt>
                <dd>{selected.source}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">Actor</dt>
                <dd>{entityText(selected.actor)}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">Target</dt>
                <dd>{entityText(selected.target)}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">Occurred</dt>
                <dd>{formatDate(selected.occurred_at)}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">Recorded</dt>
                <dd>{formatDate(selected.recorded_at)}</dd>
              </div>
            </dl>
            <div>
              <h2 className="mb-1 text-xs text-muted-foreground">Metadata</h2>
              <pre className="overflow-x-auto rounded-lg bg-muted p-3 text-xs">
                {JSON.stringify(selected.metadata, null, 2)}
              </pre>
            </div>
          </CardContent>
        </Card>
      ) : null}

      <footer className="flex items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">
          {total} {total === 1 ? "event" : "events"}
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
