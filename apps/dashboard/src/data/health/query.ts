import { useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError, UseCustomQueryOptions } from "@/data/types";
import { healthKeys } from "./keys";

export async function checkHealth(signal?: AbortSignal) {
  try {
    return await unwrap(GET("/health", { signal }));
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
    queryFn: ({ signal }) => checkHealth(signal),
    enabled,
    ...options,
  });
