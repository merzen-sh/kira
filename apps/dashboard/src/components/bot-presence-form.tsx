import { Button } from "@kira/ui/components/button";
import { Input } from "@kira/ui/components/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@kira/ui/components/select";
import { cn } from "@kira/ui/lib/utils";
import type { FormEvent } from "react";
import { useEffect, useId, useState } from "react";

import { useUpdatePresenceMutation } from "@/data/presence/mutation";
import { usePresenceQuery } from "@/data/presence/query";
import {
  type ActivitySelection,
  type BotStatus,
  MAX_ACTIVITY_TEXT,
  activityTextLength,
  presenceFromForm,
} from "./presence-form";

const STATUS_LABELS: Record<BotStatus, string> = {
  online: "Online",
  idle: "Idle",
  dnd: "Do Not Disturb",
  invisible: "Invisible (appear offline)",
};

const ACTIVITY_LABELS: Record<ActivitySelection, string> = {
  none: "No activity",
  playing: "Playing",
  watching: "Watching",
  listening: "Listening",
  competing: "Competing",
  custom: "Custom status",
};

const ACTIVITY_PLACEHOLDERS: Record<
  Exclude<ActivitySelection, "none">,
  string
> = {
  playing: "Kira dashboard",
  watching: "the logs",
  listening: "Kira FM",
  competing: "the leaderboard",
  custom: "Working on the server",
};

const STATUS_OPTIONS = Object.keys(STATUS_LABELS) as BotStatus[];
const ACTIVITY_OPTIONS = Object.keys(ACTIVITY_LABELS) as ActivitySelection[];

export interface BotPresenceFormProps {
  /** Tighter spacing for the guild dropdown. */
  compact?: boolean;
  /** Called after a successful save. */
  onSaved?: () => void;
}

export function BotPresenceForm({
  compact = false,
  onSaved,
}: BotPresenceFormProps) {
  const presence = usePresenceQuery();
  const mutation = useUpdatePresenceMutation();
  const statusId = useId();
  const activityId = useId();
  const textId = useId();
  const [status, setStatus] = useState<BotStatus>("online");
  const [activityKind, setActivityKind] =
    useState<ActivitySelection>("none");
  const [activityText, setActivityText] = useState("");
  const [formError, setFormError] = useState<string | null>(null);

  useEffect(() => {
    if (!presence.data) {
      return;
    }

    setStatus(presence.data.status);
    setActivityKind(presence.data.activity?.kind ?? "none");
    setActivityText(presence.data.activity?.text ?? "");
  }, [presence.data]);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);
    mutation.reset();

    const result = presenceFromForm(status, activityKind, activityText);

    if (result.error) {
      setFormError(result.error);
      return;
    }

    if (result.presence) {
      mutation.mutate(result.presence, {
        onSuccess: () => onSaved?.(),
      });
    }
  }

  const unavailable = presence.isPending || mutation.isPending;

  return (
    <form
      className={cn("flex flex-col", compact ? "gap-3" : "gap-4")}
      onSubmit={handleSubmit}
    >
      <div className="flex flex-col gap-1.5 text-sm font-medium">
        <label htmlFor={statusId}>Bot status</label>
        <Select
          value={status}
          onValueChange={(value) => setStatus((value ?? "online") as BotStatus)}
          items={STATUS_LABELS}
          disabled={presence.isPending}
        >
          <SelectTrigger id={statusId} className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {STATUS_OPTIONS.map((value) => (
              <SelectItem key={value} value={value}>
                {STATUS_LABELS[value]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="flex flex-col gap-1.5 text-sm font-medium">
        <label htmlFor={activityId}>Activity</label>
        <Select
          value={activityKind}
          onValueChange={(value) =>
            setActivityKind((value ?? "none") as ActivitySelection)
          }
          items={ACTIVITY_LABELS}
          disabled={presence.isPending}
        >
          <SelectTrigger id={activityId} className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {ACTIVITY_OPTIONS.map((value) => (
              <SelectItem key={value} value={value}>
                {ACTIVITY_LABELS[value]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {activityKind !== "none" ? (
        <div className="flex flex-col gap-1.5 text-sm font-medium">
          <label htmlFor={textId}>Activity text</label>
          <Input
            id={textId}
            value={activityText}
            onChange={(event) => setActivityText(event.target.value)}
            placeholder={ACTIVITY_PLACEHOLDERS[activityKind]}
          />
          <span className="text-xs font-normal text-muted-foreground">
            {activityTextLength(activityText)}/{MAX_ACTIVITY_TEXT}
          </span>
        </div>
      ) : null}

      {presence.isError ? (
        <p role="alert" className="text-sm text-destructive">
          {presence.error.message}
        </p>
      ) : null}

      {formError ? (
        <p role="alert" className="text-sm text-destructive">
          {formError}
        </p>
      ) : null}

      {mutation.isError ? (
        <p role="alert" className="text-sm text-destructive">
          {mutation.error.message}
        </p>
      ) : null}

      {mutation.isSuccess ? (
        <p role="status" className="text-sm text-emerald-600 dark:text-emerald-400">
          Presence saved.
        </p>
      ) : null}

      <div className="flex justify-end">
        <Button
          type="submit"
          size={compact ? "sm" : "default"}
          disabled={unavailable}
        >
          {mutation.isPending ? "Saving…" : "Save presence"}
        </Button>
      </div>
    </form>
  );
}
