import { beforeEach, describe, expect, it, vi } from "vitest";

import { GET } from "@/data/fetcher";
import { listReports } from "./query";

vi.mock("@/data/fetcher", () => ({
  GET: vi.fn(),
  unwrap: vi.fn(async (request: Promise<{ data: unknown }>) => (await request).data),
  toResourceError: vi.fn((error: unknown) => ({ message: String(error) })),
}));

const mockGet = vi.mocked(GET);

const response = {
  data: { reports: [], total: 0, page: 1, page_limit: 20 },
  response: new Response(null, { status: 200 }),
};

beforeEach(() => {
  mockGet.mockReset();
});

describe("listReports", () => {
  it.each([
    ["all", undefined],
    ["open", false],
    ["resolved", true],
  ])("maps %s to %s", async (filter, expected) => {
    mockGet.mockResolvedValue(response as never);

    await listReports(1, 20, "term", filter);

    expect(mockGet).toHaveBeenCalledWith("/reports", {
      params: {
        query: {
          page: 1,
          page_limit: 20,
          search: "term",
          resolved: expected,
        },
      },
    });
  });
});
