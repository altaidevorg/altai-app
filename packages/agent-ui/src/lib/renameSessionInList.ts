/**
 * Pure session title rename in a list (A6.193).
 */

export type SessionTitleMeta = { id: string; title: string; updatedAt: number };

/** Rename one session and bump updatedAt; pure, no mutation of input. */
export function renameSessionInList<T extends SessionTitleMeta>(
  sessions: readonly T[],
  id: string,
  title: string,
  now: number = Date.now(),
): T[] {
  return sessions.map((s) =>
    s.id === id ? { ...s, title, updatedAt: now } : s,
  );
}
