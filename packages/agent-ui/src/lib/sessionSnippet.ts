/**
 * Pure session history snippet / content presence helpers (A6.212).
 */

export type SnippetMessagePart = {
  type?: string;
  text?: string;
};

export type SnippetMessage = {
  role?: string;
  parts: readonly SnippetMessagePart[];
};

const CONTEXT_TAG_PATTERNS = [
  /<terminal-context[\s\S]*?<\/terminal-context>\s*/g,
  /<git-diff[\s\S]*?<\/git-diff>\s*/g,
  /<folder[\s\S]*?<\/folder>\s*/g,
  /<selection[\s\S]*?<\/selection>\s*/g,
  /<file[\s\S]*?<\/file>\s*/g,
  /<env>[\s\S]*?<\/env>\s*/gi,
];

/** Strip host-injected context blocks and collapse whitespace for list preview. */
export function cleanTranscriptSnippetText(raw: string): string {
  let cleaned = raw;
  for (const pattern of CONTEXT_TAG_PATTERNS) {
    cleaned = cleaned.replace(pattern, "");
  }
  return cleaned.replace(/\s+/g, " ").trim();
}

/**
 * Walk messages newest-first; return up to `maxLen` chars of the latest usable
 * text part, with an ellipsis when truncated. Empty when no text remains.
 */
export function extractSessionSnippet(
  messages: readonly SnippetMessage[],
  maxLen: number = 90,
): string {
  for (let i = messages.length - 1; i >= 0; i--) {
    const message = messages[i];
    // Guests with `noUncheckedIndexedAccess` (VS Code host) treat `messages[i]`
    // as possibly undefined even when iterating by length.
    if (!message) continue;
    for (const part of message.parts) {
      if (part.type !== "text") continue;
      const cleaned = cleanTranscriptSnippetText(part.text ?? "");
      if (!cleaned) continue;
      return cleaned.length > maxLen
        ? `${cleaned.slice(0, maxLen)}…`
        : cleaned;
    }
  }
  return "";
}

/**
 * True when any text part is non-empty, or any non-text part is on a user
 * message (attachment / structured kickoff without prose).
 */
export function hasConversationContent(
  messages: readonly SnippetMessage[],
): boolean {
  return messages.some((message) =>
    message.parts.some((part) => {
      if (part.type === "text") {
        return Boolean(part.text?.trim());
      }
      return message.role === "user";
    }),
  );
}
