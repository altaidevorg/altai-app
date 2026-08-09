/**
 * Pure rewind-window cut for chat messages (A6.197).
 * Keep the first N user turns (inclusive) and drop everything after.
 */

export type MessageWithRole = { role: string };

/**
 * Truncate after the Nth user message. `keep <= 0` → empty.
 * If fewer than N user messages exist, return a shallow copy of the input.
 */
export function cutThroughNthUserMessage<T extends MessageWithRole>(
  messages: readonly T[],
  keep: number,
): T[] {
  if (keep <= 0) return [];
  let seen = 0;
  for (let i = 0; i < messages.length; i++) {
    if (messages[i]!.role === "user") seen++;
    if (seen >= keep) return messages.slice(0, i + 1) as T[];
  }
  return messages.slice() as T[];
}
