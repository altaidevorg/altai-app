/**
 * Pure record/map chrome helpers (A6.208).
 */

/** Drop a single key from a record without mutating the source. */
export function omitRecordKey<T>(
  record: Readonly<Record<string, T>>,
  key: string,
): Record<string, T> {
  return Object.fromEntries(
    Object.entries(record).filter(([id]) => id !== key),
  );
}

/** Drop list items whose `id` matches. */
export function omitListItemById<T extends { id: string }>(
  items: readonly T[],
  id: string,
): T[] {
  return items.filter((item) => item.id !== id);
}
