import { useMutation, useQueryClient } from "@tanstack/react-query";

import { POST, toResourceError } from "@/data/fetcher";
import type { ResourceError } from "@/data/types";
import { authKeys } from "./keys";

async function login(pin: string): Promise<void> {
  const { error, response } = await POST("/auth/login", {
    body: { pin },
  });

  if (!response.ok) {
    throw toResourceError(error);
  }
}

export function useLoginMutation() {
  const queryClient = useQueryClient();

  return useMutation<void, ResourceError, string>({
    mutationFn: login,
    onSuccess: () => {
      void queryClient.invalidateQueries();
    },
  });
}

async function logout(): Promise<void> {
  await POST("/auth/logout");
}

export function useLogoutMutation() {
  const queryClient = useQueryClient();

  return useMutation<void, ResourceError, void>({
    mutationFn: logout,
    onSettled: () => {
      queryClient.setQueryData(authKeys.session(), false);
      queryClient.removeQueries({
        predicate: (query) => query.queryKey[0] !== authKeys.all[0],
      });
      void queryClient.invalidateQueries({ queryKey: authKeys.session() });
    },
  });
}
