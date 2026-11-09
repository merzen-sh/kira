import { queryOptions } from "@tanstack/react-query";

import { GET, unwrap } from "../fetcher";
import { exampleKeys } from "./keys";

export function exampleQueryOptions(id: string) {
  return queryOptions({
    queryKey: exampleKeys.detail(id),
    queryFn: async ({ signal }) =>
      unwrap(
        GET("/example/{id}", {
          params: { path: { id } },
          signal,
        }),
      ),
  });
}
