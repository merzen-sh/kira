import { beforeEach, describe, expect, it, vi } from "vitest";

import { GET } from "@/data/fetcher";
import { checkSession } from "./query";

vi.mock("@/data/fetcher", () => ({ GET: vi.fn() }));

const mockGet = vi.mocked(GET);

beforeEach(() => {
  mockGet.mockReset();
});

describe("checkSession", () => {
  it("reports an authenticated session", async () => {
    mockGet.mockResolvedValue({
      response: new Response(null, { status: 204 }),
    } as never);

    await expect(checkSession()).resolves.toBe(true);
  });

  it("treats a rejected session as signed out", async () => {
    mockGet.mockResolvedValue({
      response: new Response(null, { status: 401 }),
    } as never);

    await expect(checkSession()).resolves.toBe(false);
  });
});
