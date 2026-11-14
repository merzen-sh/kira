import { Badge } from "@kira/ui/components/badge";
import { Button } from "@kira/ui/components/button";
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

import { useUsersQuery } from "@/data/users/query";

export const Route = createFileRoute("/users")({
  component: UsersPage,
});

const PAGE_LIMIT = 20;

function formatDate(value: string | null | undefined) {
  return value ? new Date(value).toLocaleString() : "—";
}

function UsersPage() {
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState("");
  const [term, setTerm] = useState("");
  const [status, setStatus] = useState("");

  useEffect(() => {
    const timer = setTimeout(() => {
      setTerm(search.trim());
      setPage(1);
    }, 300);

    return () => clearTimeout(timer);
  }, [search]);

  const { data, isPending, isError, error } = useUsersQuery({
    page,
    pageLimit: PAGE_LIMIT,
    search: term,
    status,
  });
  const users = data?.users ?? [];
  const total = data?.total ?? 0;
  const lastPage = Math.max(1, Math.ceil(total / PAGE_LIMIT));

  // Another admin may change the data while this page is open.
  useEffect(() => {
    if (data && page > lastPage) {
      setPage(lastPage);
    }
  }, [data, lastPage, page]);

  return (
    <main className="flex flex-1 flex-col gap-4 p-6 md:p-8">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">Users</h1>
          <p className="text-sm text-muted-foreground">
            Guild members tracked from Discord join and leave events.
          </p>
        </div>
        <div className="flex w-full flex-wrap gap-2 sm:w-auto">
          <select
            value={status}
            onChange={(event) => {
              setStatus(event.target.value);
              setPage(1);
            }}
            aria-label="Filter by status"
            className="h-9 rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
          >
            <option value="">All statuses</option>
            <option value="active">Active</option>
            <option value="left">Left</option>
          </select>
          <Input
            type="search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search users…"
            aria-label="Search users"
            className="w-full sm:w-64"
          />
        </div>
      </header>

      {isError ? (
        <p role="alert" className="text-sm text-destructive">
          {error.message}
        </p>
      ) : null}

      <div className="rounded-xl border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Username</TableHead>
              <TableHead>Display name</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Joined</TableHead>
              <TableHead>Left</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {isPending ? (
              <TableRow>
                <TableCell colSpan={5} className="text-muted-foreground">
                  Loading users…
                </TableCell>
              </TableRow>
            ) : users.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="text-muted-foreground">
                  {term || status
                    ? "No users match these filters."
                    : "No users yet."}
                </TableCell>
              </TableRow>
            ) : (
              users.map((user) => (
                <TableRow key={user.user_id}>
                  <TableCell className="font-medium">{user.user_name}</TableCell>
                  <TableCell>{user.display_name}</TableCell>
                  <TableCell>
                    <Badge
                      variant={user.status === "active" ? "default" : "secondary"}
                    >
                      {user.status}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {formatDate(user.join_at)}
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {formatDate(user.leave_at)}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      <footer className="flex items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">
          {total} {total === 1 ? "user" : "users"}
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
