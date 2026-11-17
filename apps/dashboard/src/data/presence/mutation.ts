import type { components } from "@kira/api-types";
import { useMutation, useQueryClient } from "@tanstack/react-query";

import { PUT, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError } from "@/data/types";
import { presenceKeys } from "./keys";
import type { Presence } from "./query";

export type PresenceInput = components["schemas"]["BotPresence"];

export async function updatePresence(input: PresenceInput) {
  try {
    return await unwrap(PUT("/bot/presence", { body: input }));
  } catch (error) {
    throw toResourceError(error);
  }
}

export function useUpdatePresenceMutation() {
  const queryClient = useQueryClient();

  return useMutation<Presence, ResourceError, PresenceInput>({
    mutationFn: updatePresence,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: presenceKeys.all });
    },
  });
}
