/**
 * Pure id-index / list filter helpers shared by Inbox + Task surfaces (A6.232).
 */

/** Build a Map keyed by each row's `id`. */
export function mapById<T extends { id: string }>(
  items: readonly T[],
): Map<string, T> {
  return new Map(items.map((item) => [item.id, item]));
}

/** Keep rows never marked seen (`seenAtMs === null`). */
export function filterUnreadBySeenAtMs<T extends { seenAtMs: number | null }>(
  rows: readonly T[],
): T[] {
  return rows.filter((row) => row.seenAtMs === null);
}

/** Drop exact list members (context paths, skill names, …). */
export function removeListValue<T>(list: readonly T[], value: T): T[] {
  return list.filter((item) => item !== value);
}
