import { afterEach, describe, expect, it, vi } from "vitest";

import { formatDateTime } from "./format-date";

describe("formatDateTime", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("matches direct toLocaleString output for an ISO timestamp", () => {
    const value = "2026-11-18T12:34:56.000Z";
    expect(formatDateTime(value)).toBe(new Date(value).toLocaleString());
  });

  it("computes each unique timestamp once", () => {
    const value = "2025-06-01T00:00:00.000Z";
    const spy = vi.spyOn(Date.prototype, "toLocaleString");

    formatDateTime(value);
    formatDateTime(value);

    expect(spy).toHaveBeenCalledTimes(1);
  });
});
