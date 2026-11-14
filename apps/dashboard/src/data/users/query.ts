import { useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError, UseCustomQueryOptions } from "@/data/types";
import { userKeys, type UserListParams } from "./keys";

export async function listUsers({
  page,
  pageLimit,
  search,
  status,
}: UserListParams) {
  try {
    return await unwrap(
      GET("/guild/users", {
        params: {
          query: {
            page,
            page_limit: pageLimit,
            search: search || undefined,
            status: status || undefined,
          },
        },
      }),
    );
  } catch (error) {
    throw toResourceError(error);
  }
}

export type ResourceData = Awaited<ReturnType<typeof listUsers>>;
export type { ResourceError };

export const useUsersQuery = (
  params: UserListParams,
  {
    enabled = true,
    ...options
  }: UseCustomQueryOptions<ResourceData, ResourceError> = {},
) =>
  useQuery<ResourceData, ResourceError>({
    queryKey: userKeys.list(params),
    // Skip the abort signal: reading it lets StrictMode's dev remount cancel and refetch.
    queryFn: () => listUsers(params),
    enabled,
    ...options,
  });
