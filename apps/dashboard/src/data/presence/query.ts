import { useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError, UseCustomQueryOptions } from "@/data/types";
import { presenceKeys } from "./keys";

export async function getPresence() {
  try {
    return await unwrap(GET("/bot/presence"));
  } catch (error) {
    throw toResourceError(error);
  }
}

export type Presence = Awaited<ReturnType<typeof getPresence>>;
export type { ResourceError };

export const usePresenceQuery = <TData = Presence>({
  enabled = true,
  ...options
}: UseCustomQueryOptions<Presence, ResourceError, TData> = {}) =>
  useQuery<Presence, ResourceError, TData>({
    queryKey: presenceKeys.detail(),
    queryFn: () => getPresence(),
    enabled,
    ...options,
  });
