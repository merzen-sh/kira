export const reportKeys = {
  all: ["reports"] as const,
  list: (page: number, pageLimit: number, search: string) =>
    [...reportKeys.all, "list", page, pageLimit, search] as const,
};
