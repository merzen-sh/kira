import { useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError, UseCustomQueryOptions } from "@/data/types";
import { guildKeys } from "./keys";

export async function checkGuild() {
  try {
    return await unwrap(GET("/guild"));
  } catch (error) {
    throw toResourceError(error);
  }
}

export type ResourceData = Awaited<ReturnType<typeof checkGuild>>;
export type { ResourceError };

export const useGuildQuery = <TData = ResourceData>({
  enabled = true,
  ...options
}: UseCustomQueryOptions<ResourceData, ResourceError, TData> = {}) =>
  useQuery<ResourceData, ResourceError, TData>({
    queryKey: guildKeys.detail(),
    // Skip the abort signal: reading it lets StrictMode's dev remount cancel and refetch.
    queryFn: () => checkGuild(),
    enabled,
    refetchInterval: (query) => (query.state.data ? 60_000 : 10_000),
    ...options,
  });
