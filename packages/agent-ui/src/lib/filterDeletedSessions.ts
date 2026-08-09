/**
 * Pure permanent-delete blocklist filter (A6.190).
 */

/** Drop sessions whose ids are on the deleted blocklist. */
export function filterDeletedSessions<T extends { id: string }>(
  sessions: readonly T[],
  deletedIds: readonly string[],
): T[] {
  if (deletedIds.length === 0) return [...sessions];
  const deleted = new Set(deletedIds);
  return sessions.filter((s) => !deleted.has(s.id));
}

/** Append a session id to the deleted blocklist if missing. */
export function appendDeletedSessionId(
  deletedIds: readonly string[],
  id: string,
): string[] {
  if (deletedIds.includes(id)) return [...deletedIds];
  return [...deletedIds, id];
}
