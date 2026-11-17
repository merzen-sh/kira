import { describe, expect, it } from "vitest";

import {
  MAX_ACTIVITY_TEXT,
  activityTextLength,
  presenceFromForm,
} from "./presence-form";

describe("presenceFromForm", () => {
  it("clears activity when none is selected", () => {
    expect(presenceFromForm("online", "none", "ignored")).toEqual({
      presence: { status: "online", activity: null },
    });
  });

  it("requires text for a selected activity", () => {
    expect(presenceFromForm("idle", "playing", "   ")).toEqual({
      error: "Activity text is required.",
    });
  });

  it("rejects text longer than the shared limit", () => {
    expect(
      presenceFromForm("dnd", "watching", "x".repeat(MAX_ACTIVITY_TEXT + 1)),
    ).toEqual({
      error: `Activity text must be ${MAX_ACTIVITY_TEXT} characters or fewer.`,
    });
  });

  it("trims the activity text", () => {
    expect(presenceFromForm("invisible", "custom", "  Kira  ")).toEqual({
      presence: {
        status: "invisible",
        activity: { kind: "custom", text: "Kira" },
      },
    });
  });

  it("counts characters by Unicode code point", () => {
    expect(activityTextLength("Kira")).toBe(4);
    expect(activityTextLength("🎮")).toBe(1);
    expect(activityTextLength("🎮".repeat(MAX_ACTIVITY_TEXT))).toBe(
      MAX_ACTIVITY_TEXT,
    );
  });

  it("accepts the full limit in emoji characters", () => {
    const text = "🎮".repeat(MAX_ACTIVITY_TEXT);

    expect(presenceFromForm("online", "playing", text)).toEqual({
      presence: {
        status: "online",
        activity: { kind: "playing", text },
      },
    });
  });

  it("rejects one character over the limit in emoji", () => {
    expect(
      presenceFromForm("online", "playing", "🎮".repeat(MAX_ACTIVITY_TEXT + 1)),
    ).toEqual({
      error: `Activity text must be ${MAX_ACTIVITY_TEXT} characters or fewer.`,
    });
  });
});

