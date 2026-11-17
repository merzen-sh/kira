import { beforeEach, describe, expect, it, vi } from "vitest";

import { GET } from "@/data/fetcher";
import { getPresence } from "./query";

vi.mock("@/data/fetcher", () => ({
  GET: vi.fn(),
  unwrap: vi.fn(async (request: Promise<{ data: unknown }>) => (await request).data),
  toResourceError: vi.fn((error: unknown) => ({ message: String(error) })),
}));

const mockGet = vi.mocked(GET);

beforeEach(() => {
  mockGet.mockReset();
});

describe("getPresence", () => {
  it("requests the bot presence endpoint", async () => {
    const presence = { status: "dnd", activity: null };
    mockGet.mockResolvedValue({
      data: presence,
      response: new Response(null, { status: 200 }),
    } as never);

    await expect(getPresence()).resolves.toEqual(presence);
    expect(mockGet).toHaveBeenCalledWith("/bot/presence");
  });
});
