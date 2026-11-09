# API domain templates

Copy this directory to `src/data/<domain>/` and rename the exports. The
`health/` domain is the first real example.

## Files

- `keys.ts` — hierarchical TanStack Query keys for the domain.
- `query.ts` — `queryOptions` factories that call `GET` and unwrap the result.
- `mutation.ts` — `mutationOptions` factories that call `POST`/`PUT`/`PATCH`/
  `DELETE` and unwrap the result.

## Conventions

1. Import HTTP helpers from `../fetcher`. Never create another
   `openapi-fetch` client.
2. Use a path key from the generated `paths` type. Do not hand-write URLs.
3. Give every query a key from the domain's `keys.ts`.
4. Pass every request through `unwrap`. It normalizes network, HTTP, and
   empty-body failures into `ApiError`.
5. Keep React components out of this layer. Components consume the exported
   options with `useQuery` / `useMutation`.
6. Add cross-cutting request behavior once in `fetcher.ts` (`setAuthToken`, or
   `client.use(...)`). Do not duplicate it in domain files.

## Placeholder paths

The template paths (`/example`, `/example/{id}`) are placeholders and are
excluded from `tsc` by `apps/dashboard/tsconfig.json`. After copying a
template into a real domain, replace the placeholder with a real path key and
the copied files will be typechecked as part of `src`.
