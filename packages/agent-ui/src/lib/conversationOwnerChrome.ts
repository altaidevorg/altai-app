/**
 * Pure conversation-owner selection for Operations create forms (A6.219).
 */

/**
 * When the active chat should become the owner (missing or stale owner id),
 * return that id; otherwise `null` (no state change).
 */
export function nextConversationOwnerChatId(
  activeChatId: string | null | undefined,
  ownerChatId: string,
  sessionIds: Iterable<string>,
): string | null {
  if (!activeChatId) return null;
  const known = sessionIds instanceof Set ? sessionIds : new Set(sessionIds);
  if (!ownerChatId || !known.has(ownerChatId)) {
    return activeChatId;
  }
  return null;
}
