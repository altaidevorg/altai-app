/**
 * Pure session-list delete helpers (A6.195).
 */

/** Drop a session id from the list. */
export function removeSessionFromList<T extends { id: string }>(
  sessions: readonly T[],
  id: string,
): T[] {
  return sessions.filter((s) => s.id !== id);
}

/**
 * After delete: remaining head if the deleted row was active; otherwise keep
 * the current active id. Returns null when the list becomes empty.
 */
export function nextActiveIdAfterDelete(
  sessions: readonly { id: string }[],
  deletedId: string,
  currentActiveId: string | null | undefined,
): string | null {
  const remaining = removeSessionFromList(sessions, deletedId);
  if (remaining.length === 0) return null;
  if (currentActiveId === deletedId) return remaining[0]!.id;
  return currentActiveId ?? null;
}
