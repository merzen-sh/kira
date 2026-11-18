import { Badge } from "@kira/ui/components/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";
import { createFileRoute } from "@tanstack/react-router";
import { Server } from "lucide-react";

import { BotPresenceForm } from "@/components/bot-presence-form";
import { useGuildQuery } from "@/data/guild/query";

export const Route = createFileRoute("/server")({
  component: ServerPage,
});

function ServerPage() {
  const { data: guild, isPending, isError } = useGuildQuery();

  return (
    <main className="flex flex-1 flex-col gap-4 p-6 md:p-8">
      <header className="mx-auto w-full max-w-3xl">
        <h1 className="text-xl font-semibold">Server</h1>
        <p className="text-sm text-muted-foreground">
          Configured Discord server details and bot presence.
        </p>
      </header>

      <Card className="mx-auto w-full max-w-3xl">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            {guild?.icon_url ? (
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
            {guild?.name ?? "Discord server"}
          </CardTitle>
          <CardDescription>Details cached from the Discord gateway.</CardDescription>
        </CardHeader>

        <CardContent className="flex flex-col gap-4">
          {isPending ? (
            <p className="text-muted-foreground">Loading server details…</p>
          ) : null}

          {isError ? (
            <p role="alert" className="text-sm text-destructive">
              Server details are unavailable.
            </p>
          ) : null}

          {guild ? (
            <>
              <dl className="grid gap-3 text-sm sm:grid-cols-2">
                <div>
                  <dt className="text-muted-foreground">Server ID</dt>
                  <dd className="font-mono text-xs sm:text-sm">{guild.id}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Owner ID</dt>
                  <dd className="font-mono text-xs sm:text-sm">
                    {guild.owner_id || "—"}
                  </dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Members</dt>
                  <dd>{guild.member_count}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Online</dt>
                  <dd>{guild.presence_count ?? "—"}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Preferred locale</dt>
                  <dd>{guild.preferred_locale || "—"}</dd>
                </div>
              </dl>

              <div>
                <p className="text-sm text-muted-foreground">Features</p>
                {guild.features.length > 0 ? (
                  <div className="mt-1 flex flex-wrap gap-1">
                    {guild.features.map((feature) => (
                      <Badge key={feature} variant="secondary">
                        {feature}
                      </Badge>
                    ))}
                  </div>
                ) : (
                  <p className="mt-1 text-sm text-muted-foreground">None</p>
                )}
              </div>
            </>
          ) : null}
        </CardContent>
      </Card>

      <Card id="bot-presence" className="mx-auto w-full max-w-3xl">
        <CardHeader>
          <CardTitle>Bot presence</CardTitle>
          <CardDescription>
            Control the status and activity the bot shows in Discord.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <BotPresenceForm />
        </CardContent>
      </Card>
    </main>
  );
}
