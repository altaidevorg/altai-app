/**
 * Pure helpers for Operations → Chat deep-links (A6.100).
 * No React / vscode APIs.
 */

export type OpenChatTarget = {
  /** Owner conversation when known (task run or notification). */
  chatId?: string;
  /** Human-readable label for Chat status copy. */
  label?: string;
};

export type OpenChatFocus = OpenChatTarget & {
  /** Unique key so remounted focus opens re-apply. */
  key: number;
};

/**
 * Normalize an Operations "open chat" request. Drops empty chat ids / labels.
 */
export function buildOpenChatFocus(
  input: OpenChatTarget,
  key: number = Date.now(),
): OpenChatFocus {
  const focus: OpenChatFocus = { key };
  const chatId = input.chatId?.trim();
  if (chatId) {
    focus.chatId = chatId;
  }
  const label = input.label?.trim();
  if (label) {
    focus.label = label;
  }
  return focus;
}

/**
 * Status line for the TASK-009 chat slice when focusing a conversation.
 */
export function chatFocusStatusLine(focus: OpenChatFocus): string {
  if (focus.chatId && focus.label) {
    return `Opened from Operations · ${focus.label} · chat ${focus.chatId}`;
  }
  if (focus.chatId) {
    return `Opened from Operations · chat ${focus.chatId}`;
  }
  if (focus.label) {
    return `Opened from Operations · ${focus.label}`;
  }
  return "Opened Chat from Operations";
}
