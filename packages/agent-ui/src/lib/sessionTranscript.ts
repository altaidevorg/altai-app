/**
 * Pure helpers for formatting session transcripts into the TASK-009 chat log (A6.103).
 */

export type TranscriptMessage = {
  role: "user" | "assistant" | "system" | "tool";
  content: string;
};

const DEFAULT_MAX_LINES = 100;
const DEFAULT_MAX_CHARS = 400;

/**
 * Map a host session message into a single chat-log line.
 */
export function formatSessionMessageLine(
  message: TranscriptMessage,
  maxChars: number = DEFAULT_MAX_CHARS,
): string {
  const label =
    message.role === "user"
      ? "You"
      : message.role === "assistant"
        ? "ALTAI"
        : message.role === "system"
          ? "System"
          : "Tool";
  const body = message.content.replace(/\s+/g, " ").trim();
  if (!body) {
    return `${label}: (empty)`;
  }
  if (body.length <= maxChars) {
    return `${label}: ${body}`;
  }
  return `${label}: ${body.slice(0, maxChars)}…`;
}

/**
 * Prefer user/assistant turns for the vertical-slice log. Newest messages kept
 * when the transcript exceeds `maxLines`.
 */
export function transcriptLinesFromMessages(
  messages: readonly TranscriptMessage[],
  options?: { maxLines?: number; maxChars?: number },
): string[] {
  const maxLines = options?.maxLines ?? DEFAULT_MAX_LINES;
  const maxChars = options?.maxChars ?? DEFAULT_MAX_CHARS;
  return messages
    .filter((message) => message.role === "user" || message.role === "assistant")
    .slice(-maxLines)
    .map((message) => formatSessionMessageLine(message, maxChars));
}
