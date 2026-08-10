/**
 * Pure history toggle + mini-window control labels (A6.253).
 */

/** Tooltip / aria for the chat-history toggle control. */
export function historyToggleLabel(historyOpen: boolean): string {
  return historyOpen ? "Back to task" : "Chat sessions";
}

/** Status-bar title for open-conversation / mini-window control. */
export function miniConversationControlTitle(miniOpen: boolean): string {
  return miniOpen ? "Mini-window open" : "Open conversation";
}
