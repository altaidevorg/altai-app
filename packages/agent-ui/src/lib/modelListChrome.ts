/**
 * Pure model id list ops for favorites / recents (A6.171).
 * Host owns stores and persistence.
 */

/** Toggle `id` in a list (add when missing, remove when present). */
export function toggleIdInList(
  ids: readonly string[],
  id: string,
): string[] {
  return ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id];
}

/**
 * Move `id` to the front of recents, dedupe, and clamp to `max`.
 * Returns a new array (may equal current when already newest-first identical).
 */
export function pushRecentId(
  ids: readonly string[],
  id: string,
  max = 5,
): string[] {
  return [id, ...ids.filter((x) => x !== id)].slice(0, max);
}

/** True when two id sequences are equal element-wise. */
export function sameIdSequence(
  a: readonly string[],
  b: readonly string[],
): boolean {
  return a.length === b.length && a.every((x, i) => x === b[i]);
}
