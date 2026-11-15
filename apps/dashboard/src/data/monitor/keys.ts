export type MonitorRange = "1h" | "3h" | "24h";

export const monitorKeys = {
  all: ["monitor"] as const,
  report: (range: MonitorRange) => [...monitorKeys.all, range] as const,
};
