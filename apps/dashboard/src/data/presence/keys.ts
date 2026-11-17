export const presenceKeys = {
  all: ["presence"] as const,
  detail: () => [...presenceKeys.all, "detail"] as const,
};
