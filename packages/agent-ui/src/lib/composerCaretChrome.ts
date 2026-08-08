/**
 * Pure caret helpers for the chat composer draft (A6.82).
 * Hosts wire selection APIs; no vscode/DOM imports here.
 */

/** Prefer a finite selection index; otherwise treat caret as end of draft. */
export function resolveComposerCaret(
  selectionStart: number | null | undefined,
  valueLength: number,
): number {
  if (
    typeof selectionStart === "number" &&
    Number.isFinite(selectionStart) &&
    selectionStart >= 0
  ) {
    return Math.min(selectionStart, Math.max(0, valueLength));
  }
  return Math.max(0, valueLength);
}

/**
 * After an uncontrolled-looking controlled update, if caret state is still 0
 * but text grew, jump caret to end so `/` `#` `@` triggers fire immediately.
 */
export function advanceCaretAfterDraftChange(
  previousCaret: number,
  nextValue: string,
  previousValueLength: number,
): number {
  if (nextValue.length === 0) {
    return 0;
  }
  if (previousCaret === 0 && nextValue.length > previousValueLength) {
    return nextValue.length;
  }
  return previousCaret;
}
