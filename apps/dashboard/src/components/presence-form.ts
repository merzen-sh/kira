import type { components } from "@kira/api-types";

export type BotPresence = components["schemas"]["BotPresence"];
export type BotStatus = components["schemas"]["BotStatus"];
export type ActivityKind = components["schemas"]["ActivityKind"];
export type ActivitySelection = ActivityKind | "none";

export interface PresenceFormResult {
  presence?: BotPresence;
  error?: string;
}

export function presenceFromForm(
  status: BotStatus,
  activityKind: ActivitySelection,
  activityText: string,
): PresenceFormResult {
  if (activityKind === "none") {
    return { presence: { status, activity: null } };
  }

  const text = activityText.trim();

  if (!text) {
    return { error: "Activity text is required." };
  }

  if (text.length > 128) {
    return { error: "Activity text must be 128 characters or fewer." };
  }

  return { presence: { status, activity: { kind: activityKind, text } } };
}
