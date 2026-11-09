import { mutationOptions } from "@tanstack/react-query";

import { POST, toResourceError, unwrap } from "@/data/fetcher";
import { exampleKeys } from "./keys";

export interface ExampleMutationInput {
  name: string;
}

export function createExampleMutationOptions() {
  return mutationOptions({
    mutationKey: exampleKeys.mutations(),
    mutationFn: async (input: ExampleMutationInput) => {
      try {
        return await unwrap(
          POST("/example", {
            body: input,
          }),
        );
      } catch (error) {
        throw toResourceError(error);
      }
    },
    // onSuccess: (data, variables, _onMutateResult, context) => {
    //   context.client.invalidateQueries({ queryKey: exampleKeys.lists() });
    // },
  });
}
