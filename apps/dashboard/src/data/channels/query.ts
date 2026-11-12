import { useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError, UseCustomQueryOptions } from "@/data/types";
import { channelKeys } from "./keys";

export async function checkChannels(signal?: AbortSignal) {
  try {
    return await unwrap(GET("/guild/channels", { signal }));
  } catch (error) {
    throw toResourceError(error);
  }
}

export type ResourceData = Awaited<ReturnType<typeof checkChannels>>;
export type { ResourceError };

export const useChannelsQuery = <TData = ResourceData>({
  enabled = true,
  ...options
}: UseCustomQueryOptions<ResourceData, ResourceError, TData> = {}) =>
  useQuery<ResourceData, ResourceError, TData>({
    queryKey: channelKeys.list(),
    queryFn: ({ signal }) => checkChannels(signal),
    enabled,
    refetchInterval: 60_000,
    ...options,
  });
