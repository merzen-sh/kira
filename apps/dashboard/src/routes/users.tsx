import { Badge } from "@kira/ui/components/badge";
import { Button } from "@kira/ui/components/button";
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
import { Bot, UserCheck, UserMinus, Users } from "lucide-react";
import { useEffect, useState } from "react";

import { StatCard } from "@/components/stat-card";
import { formatDateTime } from "@/lib/format-date";
import { useUserStatsQuery, useUsersQuery } from "@/data/users/query";

export const Route = createFileRoute("/users")({
  component: UsersPage,
});

const PAGE_LIMIT = 20;

const STATUS_ITEMS = {
  all: "All statuses",
  active: "Active",
  left: "Left",
};

const KIND_ITEMS = {
  all: "All members",
  user: "Users",
  bot: "Bots",
};

function UsersPage() {
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState("");
  const [term, setTerm] = useState("");
  const [status, setStatus] = useState("");
  const [kind, setKind] = useState("all");
  const [seenLastPage, setSeenLastPage] = useState(0);

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
    kind,
  });
  const stats = useUserStatsQuery();
  const users = data?.users ?? [];
  const total = data?.total ?? 0;
  const lastPage = Math.max(1, Math.ceil(total / PAGE_LIMIT));

  // Another admin may change the data while this page is open. Clamp an
  // over-shoot page during render (React re-renders immediately without
  // committing the stale frame) instead of reacting in an effect.
  if (data && lastPage !== seenLastPage) {
    setSeenLastPage(lastPage);
    if (page > lastPage) {
      setPage(lastPage);
    }
  }

  return (
    <main className="flex flex-1 flex-col gap-4 p-6 md:p-8">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">Users</h1>
          <p className="text-sm text-muted-foreground">
            Guild members and bot accounts tracked from Discord join and leave
            events.
          </p>
        </div>
        <div className="flex w-full flex-wrap gap-2 sm:w-auto">
          <Select
            value={kind}
            onValueChange={(value) => {
              setKind(String(value ?? "all"));
              setPage(1);
            }}
            items={KIND_ITEMS}
          >
            <SelectTrigger
              aria-label="Filter by account type"
              className="w-full sm:w-36"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All members</SelectItem>
              <SelectItem value="user">Users</SelectItem>
              <SelectItem value="bot">Bots</SelectItem>
            </SelectContent>
          </Select>
          <Select
            value={status || "all"}
            onValueChange={(value) => {
              setStatus(value === "all" ? "" : String(value));
              setPage(1);
            }}
            items={STATUS_ITEMS}
          >
            <SelectTrigger
              aria-label="Filter by status"
              className="w-full sm:w-36"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All statuses</SelectItem>
              <SelectItem value="active">Active</SelectItem>
              <SelectItem value="left">Left</SelectItem>
            </SelectContent>
          </Select>
          <Input
            type="search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search members…"
            aria-label="Search members"
            className="w-full sm:w-64"
          />
        </div>
      </header>

      <section
        aria-label="User statistics"
        className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4"
      >
        <StatCard
          label="Total users"
          icon={Users}
          value={stats.data?.total}
          pending={stats.isPending}
          error={stats.isError}
        />
        <StatCard
          label="Active"
          icon={UserCheck}
          value={stats.data?.active}
          pending={stats.isPending}
          error={stats.isError}
        />
        <StatCard
          label="Left"
          icon={UserMinus}
          value={stats.data?.left}
          pending={stats.isPending}
          error={stats.isError}
        />
        <StatCard
          label="Bots"
          icon={Bot}
          value={stats.data?.bots}
          pending={stats.isPending}
          error={stats.isError}
        />
      </section>

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
              <TableHead>Type</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Joined</TableHead>
              <TableHead>Left</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {isPending ? (
              <TableRow>
                <TableCell colSpan={6} className="text-muted-foreground">
                  Loading members…
                </TableCell>
              </TableRow>
            ) : users.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-muted-foreground">
                  {term || status || kind !== "all"
                    ? "No members match these filters."
                    : "No members yet."}
                </TableCell>
              </TableRow>
            ) : (
              users.map((user) => (
                <TableRow key={user.user_id}>
                  <TableCell className="font-medium">{user.user_name}</TableCell>
                  <TableCell>{user.display_name}</TableCell>
                  <TableCell>
                    <Badge variant={user.is_bot ? "outline" : "secondary"}>
                      {user.is_bot ? "Bot" : "User"}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <Badge
                      variant={user.status === "active" ? "default" : "secondary"}
                    >
                      {user.status}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {user.join_at ? formatDateTime(user.join_at) : "—"}
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {user.leave_at ? formatDateTime(user.leave_at) : "—"}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      <footer className="flex items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">
          {total} {total === 1 ? "member" : "members"}
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
