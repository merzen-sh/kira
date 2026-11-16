export interface AuditListParams {
  page: number;
  pageLimit: number;
  search: string;
  action: string;
  actorKind: string;
  source: string;
}

export const auditKeys = {
  all: ["audit"] as const,
  lists: () => [...auditKeys.all, "list"] as const,
  list: (params: AuditListParams) => [...auditKeys.lists(), params] as const,
  facets: () => [...auditKeys.all, "facets"] as const,
};
