import { useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError, UseCustomQueryOptions } from "@/data/types";
import { healthKeys } from "./keys";

export async function checkHealth() {
  try {
    return await unwrap(GET("/health"));
  } catch (error) {
    throw toResourceError(error);
  }
}

export type ResourceData = Awaited<ReturnType<typeof checkHealth>>;
export type { ResourceError };

export const useHealthQuery = <TData = ResourceData>({
  enabled = true,
  ...options
}: UseCustomQueryOptions<ResourceData, ResourceError, TData> = {}) =>
  useQuery<ResourceData, ResourceError, TData>({
    queryKey: healthKeys.detail(),
    // Skip the abort signal: reading it lets StrictMode's dev remount cancel and refetch.
    queryFn: () => checkHealth(),
    enabled,
    ...options,
  });
