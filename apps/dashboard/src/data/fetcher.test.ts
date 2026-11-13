import { afterEach, describe, expect, it, vi } from "vitest";

import {
  ApiError,
  GET,
  setUnauthorizedHandler,
  toResourceError,
  unwrap,
} from "./fetcher";

afterEach(() => {
  setUnauthorizedHandler(undefined);
  vi.unstubAllGlobals();
});

describe("unauthorized responses", () => {
  it("notifies the app when a request returns 401", async () => {
    const handler = vi.fn();
    setUnauthorizedHandler(handler);
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response(null, { status: 401 })),
    );

    await GET("/auth/session");

    expect(handler).toHaveBeenCalledOnce();
  });
});

describe("unwrap", () => {
  it("returns response data", async () => {
    const response = new Response(null, { status: 200 });

    await expect(
      unwrap(Promise.resolve({ data: { id: "10" }, response })),
    ).resolves.toEqual({ id: "10" });
  });

  it("tags network failures as ApiError", async () => {
    const request = Promise.reject(new Error("offline"));

    await expect(unwrap(request)).rejects.toBeInstanceOf(ApiError);
  });

  it("rejects an error payload with its status", async () => {
    const response = new Response(null, { status: 404 });
    const request = Promise.resolve({ error: { error: "nope" }, response });

    await expect(unwrap(request)).rejects.toMatchObject({ status: 404 });
  });

  it("rejects an empty body", async () => {
    const response = new Response(null, { status: 204 });

    await expect(
      unwrap(Promise.resolve({ data: undefined, response })),
    ).rejects.toThrow("API response contained no data");
  });
});

describe("toResourceError", () => {
  it("uses the API error payload message", () => {
    const error = new ApiError("failed", { payload: { error: "boom" } });

    expect(toResourceError(error)).toEqual({ message: "boom" });
  });

  it("falls back for unknown values", () => {
    expect(toResourceError("nope")).toEqual({ message: "unknown API error" });
  });
});
