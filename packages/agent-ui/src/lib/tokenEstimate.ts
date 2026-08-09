/**
 * Pure token estimate + cleared-output markers for transcript prune (A6.145).
 * Hosts implement full prune walks; these helpers are shared facts/metrics.
 */

/** Marker shape stored in a pruned tool part's output field. */
export const CLEARED_OUTPUT: { cleared: true } = { cleared: true };

/** Marker text rendered in place of a cleared tool output. */
export const CLEARED_TOOL_OUTPUT_TEXT = "[Old tool result content cleared]";

/** True when a part's output has already been cleared by a prior prune pass. */
export function isClearedOutput(output: unknown): boolean {
  return (
    typeof output === "object" &&
    output !== null &&
    (output as { cleared?: unknown }).cleared === true
  );
}

/**
 * Rough chars→tokens estimate (~4 chars/token). Advisory only — not a real
 * tokenizer; prune budgets tolerate ~25% error.
 */
export function estimateTokens(s: string): number {
  return Math.ceil(s.length / 4);
}
