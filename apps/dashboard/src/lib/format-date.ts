const cache = new Map<string, string>();

/** Format an ISO timestamp for display, computing each unique value once. */
export function formatDateTime(value: string): string {
  let formatted = cache.get(value);
  if (formatted === undefined) {
    formatted = new Date(value).toLocaleString();
    cache.set(value, formatted);
  }
  return formatted;
}
