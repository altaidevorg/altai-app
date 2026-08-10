/**
 * Pure Chat History popover chrome (A6.259).
 */

export const CHAT_HISTORY_CONTROL_LABEL = "Chat history";

export const CHAT_HISTORY_SEARCH_ARIA_LABEL = "Search chat history";

export const CHAT_HISTORY_SEARCH_PLACEHOLDER = "Search chat history…";

/** Default title when a session has no custom name. */
export const SESSION_UNTITLED_TITLE = "New chat";

/** Empty list body when history/filter returns zero groups. */
export function chatHistoryEmptyMessage(hasSearchQuery: boolean): string {
  return hasSearchQuery ? "No chats match." : "No chats yet.";
}
