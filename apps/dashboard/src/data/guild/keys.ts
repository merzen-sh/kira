export const guildKeys = {
  all: ["guild"] as const,
  detail: () => [...guildKeys.all, "detail"] as const,
};
