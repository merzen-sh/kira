import type { components } from "@kira/api-types";
import { queryOptions } from "@tanstack/react-query";

import { GET, unwrap } from "../fetcher";
import { healthKeys } from "./keys";

export type HealthResponse = components["schemas"]["HealthResponse"];

export function healthQueryOptions() {
  return queryOptions({
    queryKey: healthKeys.detail(),
    queryFn: async ({ signal }): Promise<HealthResponse> =>
      unwrap(GET("/health", { signal })),
  });
}
