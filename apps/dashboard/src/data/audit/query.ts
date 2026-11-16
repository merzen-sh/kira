import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { GET, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError, UseCustomQueryOptions } from "@/data/types";
import { auditKeys, type AuditListParams } from "./keys";

export async function listAuditEvents({
  page,
  pageLimit,
  search,
  action,
  actorKind,
}: AuditListParams) {
  try {
    return await unwrap(
      GET("/audit/events", {
        params: {
          query: {
            page,
            page_limit: pageLimit,
            search: search || undefined,
            action: action || undefined,
            actor_kind: actorKind || undefined,
          },
        },
      }),
    );
  } catch (error) {
    throw toResourceError(error);
  }
}

export async function getAuditFacets() {
  try {
    return await unwrap(GET("/audit/facets"));
  } catch (error) {
    throw toResourceError(error);
  }
}

export type AuditEvent = Awaited<
  ReturnType<typeof listAuditEvents>
>["events"][number];
export type AuditFacetsData = Awaited<ReturnType<typeof getAuditFacets>>;
export type { ResourceError };

export const useAuditEventsQuery = (
  params: AuditListParams,
  {
    enabled = true,
    ...options
  }: UseCustomQueryOptions<
    Awaited<ReturnType<typeof listAuditEvents>>,
    ResourceError
  > = {},
) =>
  useQuery<
    Awaited<ReturnType<typeof listAuditEvents>>,
    ResourceError
  >({
    queryKey: auditKeys.list(params),
    // Skip the abort signal: reading it lets StrictMode's dev remount cancel and refetch.
    queryFn: () => listAuditEvents(params),
    placeholderData: keepPreviousData,
    enabled,
    ...options,
  });

export const useAuditFacetsQuery = ({
  enabled = true,
  ...options
}: UseCustomQueryOptions<AuditFacetsData, ResourceError> = {}) =>
  useQuery<AuditFacetsData, ResourceError>({
    queryKey: auditKeys.facets(),
    // Skip the abort signal: reading it lets StrictMode's dev remount cancel and refetch.
    queryFn: () => getAuditFacets(),
    enabled,
    ...options,
  });
