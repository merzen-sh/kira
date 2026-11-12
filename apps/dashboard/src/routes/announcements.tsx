import { Button } from "@kira/ui/components/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";
import { createFileRoute } from "@tanstack/react-router";
import type { FormEvent } from "react";
import { useState } from "react";

import { useChannelsQuery } from "@/data/channels/query";
import { useSendMessageMutation } from "@/data/messages/mutation";

export const Route = createFileRoute("/announcements")({
  component: AnnouncementsPage,
});

const SENDABLE_KINDS = new Set([
  "text",
  "news",
  "public_thread",
  "private_thread",
]);

type Mode = "announcement" | "embed";

function AnnouncementsPage() {
  const { data: channels, isPending, isError } = useChannelsQuery();
  const mutation = useSendMessageMutation();
  const [mode, setMode] = useState<Mode>("announcement");
  const [selectedId, setSelectedId] = useState("");
  const [content, setContent] = useState("");
  const [embedJson, setEmbedJson] = useState("");
  const [formError, setFormError] = useState<string | null>(null);

  const sendable = (channels ?? []).filter((channel) =>
    SENDABLE_KINDS.has(channel.kind),
  );
  const channelId = sendable.some((channel) => channel.id === selectedId)
    ? selectedId
    : (sendable[0]?.id ?? "");
  const channel = sendable.find((candidate) => candidate.id === channelId);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);
    mutation.reset();

    if (!channelId) {
      setFormError("Select a channel first.");
      return;
    }

    if (mode === "announcement") {
      const text = content.trim();

      if (!text) {
        setFormError("Write the announcement text.");
        return;
      }

      mutation.mutate(
        { channelId, message: { content: text } },
        { onSuccess: () => setContent("") },
      );
      return;
    }

    let message: unknown;
    try {
      message = JSON.parse(embedJson);
    } catch {
      setFormError("Message JSON is not valid.");
      return;
    }

    if (typeof message !== "object" || message === null || Array.isArray(message)) {
      setFormError("Message JSON must be an object.");
      return;
    }

    mutation.mutate(
      { channelId, message: message as Record<string, unknown> },
      { onSuccess: () => setEmbedJson("") },
    );
  }

  return (
    <main className="flex flex-1 justify-center p-6 md:p-8">
      <Card className="w-full max-w-2xl">
        <CardHeader>
          <CardTitle>Announcements</CardTitle>
          <CardDescription>
            Post an announcement or a raw Discord message payload to a channel.
          </CardDescription>
        </CardHeader>

        <CardContent>
          <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
            <label className="flex flex-col gap-1.5 text-sm font-medium">
              Channel
              <select
                value={channelId}
                onChange={(event) => setSelectedId(event.target.value)}
                disabled={isPending || isError || sendable.length === 0}
                className="h-9 rounded-lg border border-input bg-background px-3 text-sm font-normal text-foreground outline-none transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50 dark:bg-input/30"
              >
                {sendable.length === 0 ? (
                  <option value="">
                    {isPending
                      ? "Loading channels…"
                      : isError
                        ? "Channels unavailable"
                        : "No text channels available"}
                  </option>
                ) : (
                  sendable.map((candidate) => (
                    <option
                      key={candidate.id}
                      value={candidate.id}
                      className="bg-background text-foreground"
                    >
                      # {candidate.name}
                    </option>
                  ))
                )}
              </select>
            </label>

            <fieldset className="flex flex-wrap gap-4">
              <legend className="sr-only">Message type</legend>
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="radio"
                  name="message-mode"
                  checked={mode === "announcement"}
                  onChange={() => {
                    setMode("announcement");
                    setFormError(null);
                  }}
                  className="accent-primary"
                />
                Announcement
              </label>
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="radio"
                  name="message-mode"
                  checked={mode === "embed"}
                  onChange={() => {
                    setMode("embed");
                    setFormError(null);
                  }}
                  className="accent-primary"
                />
                Raw JSON
              </label>
            </fieldset>

            {mode === "announcement" ? (
              <label className="flex flex-col gap-1.5 text-sm font-medium">
                Message
                <textarea
                  value={content}
                  onChange={(event) => setContent(event.target.value)}
                  maxLength={2000}
                  rows={5}
                  placeholder="What should the server know?"
                  className="min-h-28 rounded-lg border border-input bg-background px-3 py-2 text-sm font-normal text-foreground outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"
                />
              </label>
            ) : (
              <label className="flex flex-col gap-1.5 text-sm font-medium">
                Message JSON
                <textarea
                  value={embedJson}
                  onChange={(event) => setEmbedJson(event.target.value)}
                  rows={10}
                  spellCheck={false}
                  placeholder={'{\n  "content": "Optional text",\n  "embeds": [\n    { "title": "Update", "description": "Details here" }\n  ]\n}'}
                  className="min-h-48 rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm font-normal text-foreground outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"
                />
              </label>
            )}

            {formError ? (
              <p className="text-sm text-destructive">{formError}</p>
            ) : null}

            {mutation.isError ? (
              <p className="text-sm text-destructive">{mutation.error.message}</p>
            ) : null}

            {mutation.isSuccess ? (
              <p className="text-sm text-emerald-600 dark:text-emerald-400">
                Message posted{channel ? ` to # ${channel.name}` : ""}.
              </p>
            ) : null}

            <div className="flex justify-end">
              <Button
                type="submit"
                disabled={mutation.isPending || !channelId}
              >
                {mutation.isPending ? "Posting…" : "Post message"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </main>
  );
}
