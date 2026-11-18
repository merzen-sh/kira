import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@kira/ui/components/popover";
import { cn } from "@kira/ui/lib/utils";
import { Link } from "@tanstack/react-router";
import ChevronDown from "lucide-react/icons/chevron-down";
import ChevronRight from "lucide-react/icons/chevron-right";
import Server from "lucide-react/icons/server";
import { useState } from "react";

import { BotPresenceForm } from "@/components/bot-presence-form";
import { useGuildQuery } from "@/data/guild/query";

export function GuildMenu() {
  const { data: guild, isError } = useGuildQuery();
  const [open, setOpen] = useState(false);
  const label =
    guild?.name ?? (isError ? "Server unavailable" : "Loading server…");
  const counts = guild
    ? `${guild.member_count} members${
        guild.presence_count != null ? ` · ${guild.presence_count} online` : ""
      }`
    : null;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        aria-label={guild ? `${guild.name} menu` : "Server menu"}
        className="flex min-w-0 items-center gap-2 rounded-md py-1 pr-2 transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none"
      >
        <GuildIcon iconUrl={guild?.icon_url} />
        <span className="min-w-0 truncate text-sm font-medium">{label}</span>
        <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
      </PopoverTrigger>

      <PopoverContent align="start" sideOffset={6} className="w-80 p-0">
        <div className="flex min-w-0 items-center gap-2 border-b border-border/60 p-3">
          <GuildIcon iconUrl={guild?.icon_url} />
          <div className="min-w-0">
            <p className="truncate text-sm font-medium">{label}</p>
            <p className="text-xs text-muted-foreground">
              {counts ?? "Guild details unavailable"}
            </p>
          </div>
        </div>

        <div className="p-3">
          <BotPresenceForm compact />
        </div>

        <div className="border-t border-border/60 p-1">
          <Link
            to="/server"
            onClick={() => setOpen(false)}
            className="flex items-center justify-between rounded-md px-2 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
          >
            Server details
            <ChevronRight className="size-4" />
          </Link>
        </div>
      </PopoverContent>
    </Popover>
  );
}

function GuildIcon({
  iconUrl,
  className,
}: {
  iconUrl?: string | null;
  className?: string;
}) {
  if (iconUrl) {
    return (
      <img
        src={iconUrl}
        alt=""
        className={cn(
          "size-7 shrink-0 rounded-full object-cover ring-1 ring-border",
          className,
        )}
      />
    );
  }

  return (
    <span
      className={cn(
        "flex size-7 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground",
        className,
      )}
    >
      <Server className="size-4" />
    </span>
  );
}
