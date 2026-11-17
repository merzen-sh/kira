import type { components } from "@kira/api-types";

export type BotPresence = components["schemas"]["BotPresence"];
export type BotStatus = components["schemas"]["BotStatus"];
export type ActivityKind = components["schemas"]["ActivityKind"];
export type ActivitySelection = ActivityKind | "none";

export const MAX_ACTIVITY_TEXT = 128;

export function activityTextLength(text: string): number {
  return Array.from(text).length;
}

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

  if (activityTextLength(text) > MAX_ACTIVITY_TEXT) {
    return {
      error: `Activity text must be ${MAX_ACTIVITY_TEXT} characters or fewer.`,
    };
  }

  return { presence: { status, activity: { kind: activityKind, text } } };
}
