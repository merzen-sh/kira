import { beforeEach, describe, expect, it, vi } from "vitest";

import { PUT } from "@/data/fetcher";
import { updatePresence } from "./mutation";

vi.mock("@/data/fetcher", () => ({
  PUT: vi.fn(),
  unwrap: vi.fn(async (request: Promise<{ data: unknown }>) => (await request).data),
  toResourceError: vi.fn((error: unknown) => ({ message: String(error) })),
}));

const mockPut = vi.mocked(PUT);

beforeEach(() => {
  mockPut.mockReset();
});

describe("updatePresence", () => {
  it("puts the presence payload", async () => {
    const presence = {
      status: "dnd",
      activity: { kind: "watching", text: "Kira" },
    } as const;
    mockPut.mockResolvedValue({
      data: presence,
      response: new Response(null, { status: 200 }),
    } as never);

    await expect(updatePresence(presence)).resolves.toEqual(presence);
    expect(mockPut).toHaveBeenCalledWith("/bot/presence", { body: presence });
  });
});
