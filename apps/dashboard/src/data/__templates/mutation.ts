import { mutationOptions } from "@tanstack/react-query";

import { POST, unwrap } from "../fetcher";
import { exampleKeys } from "./keys";

export interface ExampleMutationInput {
  name: string;
}

export function createExampleMutationOptions() {
  return mutationOptions({
    mutationKey: exampleKeys.mutations(),
    mutationFn: async (input: ExampleMutationInput) =>
      unwrap(
        POST("/example", {
          body: input,
        }),
      ),
    // onSuccess: (data, variables, _onMutateResult, context) => {
    //   context.client.invalidateQueries({ queryKey: exampleKeys.lists() });
    // },
  });
}
