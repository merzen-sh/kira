import type { UseQueryOptions } from "@tanstack/react-query";

/** Shared shape for errors surfaced to query and mutation consumers. */
export interface ResourceError {
  message: string;
}

/**
 * Query options with the key and fetch function owned by the domain hook.
 */
export type UseCustomQueryOptions<
  TQueryFnData,
  TError,
  TData = TQueryFnData,
> = Omit<
  UseQueryOptions<TQueryFnData, TError, TData>,
  "queryKey" | "queryFn"
>;
