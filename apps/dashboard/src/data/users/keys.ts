export interface UserListParams {
  page: number;
  pageLimit: number;
  search: string;
  status: string;
}

export const userKeys = {
  all: ["users"] as const,
  list: (params: UserListParams) => [...userKeys.all, "list", params] as const,
};
