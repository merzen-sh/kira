import { describe, expect, it } from "vitest";

import { presenceFromForm } from "./presence-form";

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

  it("rejects text longer than 128 characters", () => {
    expect(presenceFromForm("dnd", "watching", "x".repeat(129))).toEqual({
      error: "Activity text must be 128 characters or fewer.",
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
});
