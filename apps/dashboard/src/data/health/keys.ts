export const healthKeys = {
  all: ["health"] as const,
  detail: () => [...healthKeys.all, "detail"] as const,
};
