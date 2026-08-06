/**
 * Pure helpers for edit-and-resend of user turns in the chat transcript.
 * Wave 4 / A6.19
 *
 * Editing user turn N discards that turn and everything after it
 * (keep_user_messages = N - 1), then starts a fresh run with the edited text.
 */

/** Parse `user:N` turn ids (N is 0-based keep count or 1-based message turn). */
export function parseUserTurnId(messageId: string): number | null {
  const match = /^user:(\d+)$/.exec(messageId.trim());
  if (!match) {
    return null;
  }
  const n = Number(match[1]);
  return Number.isSafeInteger(n) && n >= 0 ? n : null;
}

/**
 * Message id to pass to `sessions.truncate` before resending an edited turn.
 * Editing turn 1 → `user:0` (wipe). Editing turn 2 → `user:1` (keep first only).
 */
export function truncateBoundaryForEdit(userTurn: number): string | null {
  if (!Number.isSafeInteger(userTurn) || userTurn < 1) {
    return null;
  }
  return `user:${userTurn - 1}`;
}

/** Drop a user turn and all following messages in a host display list. */
export function truncateDisplayAfterUserTurn<T extends { role: string }>(
  messages: readonly T[],
  userTurn: number,
): T[] {
  if (userTurn < 1) {
    return [];
  }
  let seen = 0;
  const next: T[] = [];
  for (const message of messages) {
    if (message.role === "user") {
      seen += 1;
      if (seen >= userTurn) {
        break;
      }
    }
    next.push(message);
  }
  return next;
}

export function canEditUserMessage(input: {
  role: string;
  canTruncate: boolean;
  canStartRun: boolean;
  runActive: boolean;
}): boolean {
  return (
    input.role === "user" &&
    input.canTruncate &&
    input.canStartRun &&
    !input.runActive
  );
}

/** Re-number user messages as `user:1`..`user:N` (host may supply richer rows). */
export function renumberUserTurnIds<T extends { id: string; role: string }>(
  messages: readonly T[],
): T[] {
  let userTurn = 0;
  return messages.map((message) => {
    if (message.role !== "user") {
      return message;
    }
    userTurn += 1;
    return { ...message, id: `user:${userTurn}` };
  });
}
