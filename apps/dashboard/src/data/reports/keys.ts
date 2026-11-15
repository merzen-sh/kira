export interface ReportListParams {
  page: number;
  pageLimit: number;
  search: string;
}

export const reportKeys = {
  all: ["reports"] as const,
  list: (params: ReportListParams) =>
    [...reportKeys.all, "list", params] as const,
  stats: () => [...reportKeys.all, "stats"] as const,
};
