/**
 * Pure session id → title map for Operations chrome labels (A6.216+).
 * A6.249 adds ordered id list projection.
 */

export type SessionTitleSource = {
  id: string;
  title: string;
};

/** Ordered list of session ids. */
export function sessionIds(
  sessions: readonly { id: string }[],
): string[] {
  return sessions.map((session) => session.id);
}

/** Build a stable Map of session id → title. */
export function sessionTitleMap(
  sessions: readonly SessionTitleSource[],
): Map<string, string> {
  return new Map(sessions.map((session) => [session.id, session.title]));
}

/** Build a Set of known session ids. */
export function sessionIdSet(
  sessions: readonly { id: string }[],
): Set<string> {
  return new Set(sessionIds(sessions));
}
