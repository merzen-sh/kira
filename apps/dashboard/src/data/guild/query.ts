import { useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError, UseCustomQueryOptions } from "@/data/types";
import { guildKeys } from "./keys";

export async function checkGuild(signal?: AbortSignal) {
  try {
    return await unwrap(GET("/guild", { signal }));
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
    queryFn: ({ signal }) => checkGuild(signal),
    enabled,
    refetchInterval: (query) => (query.state.data ? 60_000 : 10_000),
    ...options,
  });
