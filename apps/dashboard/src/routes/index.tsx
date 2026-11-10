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
