# API domain templates

Copy this directory to `src/data/<domain>/` and rename the exports. The
`health/` domain is the first real example.

## Files

- `keys.ts` — hierarchical TanStack Query keys for the domain.
- `query.ts` — an async fetch function plus a typed `use...Query` hook.
- `mutation.ts` — a typed `mutationOptions` factory.

## Conventions

1. Import HTTP helpers from `../fetcher`. Never create another
   `openapi-fetch` client.
2. Use a path key from the generated `paths` type. Do not hand-write URLs.
3. Wrap every request in `toResourceError` so query and mutation consumers
   receive the shared `ResourceError` shape.
4. Export `ResourceData` from the async function's return type. Do not
   duplicate response types by hand.
5. Give every query a key from the domain's `keys.ts`.
6. Use `UseCustomQueryOptions` from `../types` for hook options; the hook owns
   `queryKey` and `queryFn`.
7. Keep React components out of this layer. Components consume the exported
   hooks or mutation options.
8. Add cross-cutting request behavior once in `fetcher.ts` (`setAuthToken`, or
   `client.use(...)`). Do not duplicate it in domain files.

## Query shape

```ts
export async function getExample(id: string, signal?: AbortSignal) {
  try {
    return await unwrap(GET("/example/{id}", { params: { path: { id } }, signal }));
  } catch (error) {
    throw toResourceError(error);
  }
}

export type ResourceData = Awaited<ReturnType<typeof getExample>>;
export type ResourceError = { message: string };

export const useExampleQuery = <TData = ResourceData>(
  id: string,
  { enabled = true, ...options }: UseCustomQueryOptions<ResourceData, ResourceError, TData> = {},
) =>
  useQuery<ResourceData, ResourceError, TData>({
    queryKey: exampleKeys.detail(id),
    queryFn: ({ signal }) => getExample(id, signal),
    enabled,
    ...options,
  });
```

## Mutation shape

```ts
export function createExampleMutationOptions() {
  return mutationOptions({
    mutationKey: exampleKeys.mutations(),
    mutationFn: async (input: ExampleMutationInput) => {
      try {
        return await unwrap(POST("/example", { body: input }));
      } catch (error) {
        throw toResourceError(error);
      }
    },
  });
}
```

## Placeholder paths

The template paths (`/example`, `/example/{id}`) are placeholders and are
excluded from `tsc` by `apps/dashboard/tsconfig.json`. After copying a
template into a real domain, replace the placeholder with a real path key and
the copied files will be typechecked as part of `src`.
