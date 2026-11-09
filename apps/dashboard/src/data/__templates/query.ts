import { useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { UseCustomQueryOptions } from "@/data/types";
import { exampleKeys } from "./keys";

export async function getExample(id: string, signal?: AbortSignal) {
  try {
    return await unwrap(
      GET("/example/{id}", {
        params: { path: { id } },
        signal,
      }),
    );
  } catch (error) {
    throw toResourceError(error);
  }
}

export type ResourceData = Awaited<ReturnType<typeof getExample>>;
export type ResourceError = { message: string };

export const useExampleQuery = <TData = ResourceData>(
  id: string,
  {
    enabled = true,
    ...options
  }: UseCustomQueryOptions<ResourceData, ResourceError, TData> = {},
) =>
  useQuery<ResourceData, ResourceError, TData>({
    queryKey: exampleKeys.detail(id),
    queryFn: ({ signal }) => getExample(id, signal),
    enabled,
    ...options,
  });
