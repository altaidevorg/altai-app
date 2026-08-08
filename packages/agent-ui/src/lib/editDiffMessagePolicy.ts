/**
 * Pure edit-diff bubble helpers for flat display transcripts (A6.67).
 * Hosts pass role + optional before/after text (no host-specific DTO imports).
 */

export type EditDiffMessageLike = {
  role?: string;
  diffOriginalText?: string;
  diffModifiedText?: string;
};

export function isEditDiffMessage(message: EditDiffMessageLike): boolean {
  return (
    message.role === "tool" &&
    message.diffOriginalText !== undefined &&
    message.diffModifiedText !== undefined
  );
}

/** Count tool rows that carry before/after text for review. */
export function countPendingEditDiffs(
  messages: readonly EditDiffMessageLike[],
): number {
  return messages.filter(isEditDiffMessage).length;
}

/** Index of the most recent edit_diff bubble, or -1. */
export function lastEditDiffMessageIndex(
  messages: readonly EditDiffMessageLike[],
): number {
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    const message = messages[i];
    if (message && isEditDiffMessage(message)) {
      return i;
    }
  }
  return -1;
}

/** Most recent edit_diff bubble, or null. */
export function lastEditDiffMessage<T extends EditDiffMessageLike>(
  messages: readonly T[],
): T | null {
  const index = lastEditDiffMessageIndex(messages);
  return index >= 0 ? (messages[index] ?? null) : null;
}
