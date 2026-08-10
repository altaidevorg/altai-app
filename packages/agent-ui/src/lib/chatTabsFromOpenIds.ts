/**
 * Pure open-chat-id → ChatTabStrip tab projection (A6.238).
 */

export type SessionTabSource = {
  id: string;
  title: string;
};

/**
 * Resolve ordered open chat ids against the host session list.
 * Missing ids are skipped; order follows `openChatIds`.
 */
export function chatTabsFromOpenIds(
  openChatIds: readonly string[],
  sessions: readonly SessionTabSource[],
): Array<{ id: string; title: string }> {
  const byId = new Map(sessions.map((session) => [session.id, session]));
  const tabs: Array<{ id: string; title: string }> = [];
  for (const id of openChatIds) {
    const session = byId.get(id);
    if (!session) continue;
    tabs.push({ id: session.id, title: session.title });
  }
  return tabs;
}
