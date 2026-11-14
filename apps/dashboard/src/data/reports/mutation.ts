import type { components } from "@kira/api-types";
import { useMutation, useQueryClient } from "@tanstack/react-query";

import { DELETE, PATCH, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError } from "@/data/types";
import { reportKeys } from "./keys";

type Report = components["schemas"]["Report"];

export interface ResolveReportInput {
  id: number;
  resolved: boolean;
}

export function useResolveReportMutation() {
  const queryClient = useQueryClient();

  return useMutation<Report, ResourceError, ResolveReportInput>({
    mutationFn: async ({ id, resolved }: ResolveReportInput) => {
      try {
        return await unwrap(
          PATCH("/reports/{id}", {
            params: { path: { id } },
            body: { resolved },
          }),
        );
      } catch (error) {
        throw toResourceError(error);
      }
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: reportKeys.all });
    },
  });
}

export function useDeleteReportMutation() {
  const queryClient = useQueryClient();

  return useMutation<void, ResourceError, number>({
    mutationFn: async (id: number) => {
      const { error, response } = await DELETE("/reports/{id}", {
        params: { path: { id } },
      });

      if (!response.ok) {
        throw toResourceError(error);
      }
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: reportKeys.all });
    },
  });
}
