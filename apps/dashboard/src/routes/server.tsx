import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/server")({
  component: ServerPage,
});

function ServerPage() {
  return (
    <main className="flex-1 p-6 md:p-8">
      <Card className="max-w-2xl">
        <CardHeader>
          <CardTitle>Server</CardTitle>
          <CardDescription>Configured Discord server details.</CardDescription>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground">
          Guild details from the Kira API will be shown here.
        </CardContent>
      </Card>
    </main>
  );
}
