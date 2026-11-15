import { useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError, UseCustomQueryOptions } from "@/data/types";
import { monitorKeys, type MonitorRange } from "./keys";

export async function getMonitor(range: MonitorRange) {
  try {
    return await unwrap(GET("/monitor", { params: { query: { range } } }));
  } catch (error) {
    throw toResourceError(error);
  }
}

export type MonitorData = Awaited<ReturnType<typeof getMonitor>>;
export type MonitorSample = MonitorData["history"][number];
export type { MonitorRange, ResourceError };

export const useMonitorQuery = (
  range: MonitorRange,
  {
    enabled = true,
    ...options
  }: UseCustomQueryOptions<MonitorData, ResourceError> = {},
) =>
  useQuery<MonitorData, ResourceError>({
    queryKey: monitorKeys.report(range),
    // Skip the abort signal: reading it lets StrictMode's dev remount cancel and refetch.
    queryFn: () => getMonitor(range),
    enabled,
    refetchInterval: 10_000,
    ...options,
  });
