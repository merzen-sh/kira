import type { components } from "@kira/api-types";
import { useMutation } from "@tanstack/react-query";

import { POST, toResourceError, unwrap } from "@/data/fetcher";
import type { ResourceError } from "@/data/types";

type SentMessage = components["schemas"]["SentMessage"];

export interface SendMessageInput {
  channelId: string;
  /** Raw Discord message payload, forwarded unchanged. */
  message: Record<string, unknown>;
}

export function useSendMessageMutation() {
  return useMutation<SentMessage, ResourceError, SendMessageInput>({
    mutationFn: async ({ channelId, message }: SendMessageInput) => {
      try {
        return await unwrap(
          POST("/guild/channels/{channel_id}/messages", {
            params: { path: { channel_id: channelId } },
            body: message,
          }),
        );
      } catch (error) {
        throw toResourceError(error);
      }
    },
  });
}
