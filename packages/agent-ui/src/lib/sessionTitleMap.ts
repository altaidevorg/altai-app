/**
 * Pure session id → title map for Operations chrome labels (A6.216).
 */

export type SessionTitleSource = {
  id: string;
  title: string;
};

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
  return new Set(sessions.map((session) => session.id));
}
