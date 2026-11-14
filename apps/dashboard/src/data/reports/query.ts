import { useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError, UseCustomQueryOptions } from "@/data/types";
import { reportKeys } from "./keys";

export async function listReports(page: number, pageLimit: number, search: string) {
  try {
    return await unwrap(
      GET("/reports", {
        params: {
          query: {
            page,
            page_limit: pageLimit,
            search: search || undefined,
          },
        },
      }),
    );
  } catch (error) {
    throw toResourceError(error);
  }
}

export type ResourceData = Awaited<ReturnType<typeof listReports>>;
export type { ResourceError };

export const useReportsQuery = (
  page: number,
  pageLimit: number,
  search: string,
  {
    enabled = true,
    ...options
  }: UseCustomQueryOptions<ResourceData, ResourceError> = {},
) =>
  useQuery<ResourceData, ResourceError>({
    queryKey: reportKeys.list(page, pageLimit, search),
    // Skip the abort signal: reading it lets StrictMode's dev remount cancel and refetch.
    queryFn: () => listReports(page, pageLimit, search),
    enabled,
    ...options,
  });
