import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
});

function SettingsPage() {
  return (
    <main className="flex-1 p-6 md:p-8">
      <Card className="max-w-2xl">
        <CardHeader>
          <CardTitle>Settings</CardTitle>
          <CardDescription>Dashboard preferences.</CardDescription>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground">
          Dashboard settings will appear here.
        </CardContent>
      </Card>
    </main>
  );
}
