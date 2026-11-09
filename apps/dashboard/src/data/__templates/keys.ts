export const exampleKeys = {
  all: ["example"] as const,
  lists: () => [...exampleKeys.all, "list"] as const,
  list: (filters: Record<string, unknown>) =>
    [...exampleKeys.lists(), filters] as const,
  details: () => [...exampleKeys.all, "detail"] as const,
  detail: (id: string) => [...exampleKeys.details(), id] as const,
  mutations: () => [...exampleKeys.all, "mutation"] as const,
};
