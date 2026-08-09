/**
 * Pure session-list insert after the active tab (A6.192).
 */

/** Insert `meta` immediately after the active session (or append). */
export function insertSessionAfterActive<T extends { id: string }>(
  sessions: readonly T[],
  activeId: string | null | undefined,
  meta: T,
): T[] {
  const activeIdx = activeId
    ? sessions.findIndex((s) => s.id === activeId)
    : -1;
  if (activeIdx === -1) return [...sessions, meta];
  return [
    ...sessions.slice(0, activeIdx + 1),
    meta,
    ...sessions.slice(activeIdx + 1),
  ];
}
