/**
 * Display/persistence-only prune for the chat transcript.
 *
 * Prunes completed tool outputs from older transcript turns:
 * that fall outside a trailing recency-token budget are collapsed to a small
 * `{ cleared: true }` marker in the persisted thread. The model's own context
 * is the runtime's responsibility (its native compaction already prunes) —
 * this is purely a DOM/disk footprint optimization so a long chat doesn't
 * bloat `altai-ai-sessions.json` and the rendered transcript.
 *
 * Pure functions only — no store side effects. Tests live alongside.
 */
import type { UIMessage } from "ai";
import {
  CLEARED_OUTPUT as CLEARED_OUTPUT_SHARED,
  CLEARED_TOOL_OUTPUT_TEXT as CLEARED_TOOL_OUTPUT_TEXT_SHARED,
  isClearedOutput as isClearedOutputShared,
  estimateTokens as estimateTokensShared,
  pruneOldToolOutputs as pruneOldToolOutputsShared,
} from "@altai/agent-ui";

/** Marker text rendered in place of a cleared tool output. */
export const CLEARED_TOOL_OUTPUT_TEXT = CLEARED_TOOL_OUTPUT_TEXT_SHARED;

/** Marker shape stored in the persisted part's `output` field. */
export const CLEARED_OUTPUT: { cleared: true } = CLEARED_OUTPUT_SHARED;

/** True when a part's output has already been cleared by a prior prune pass. */
export function isClearedOutput(output: unknown): boolean {
  return isClearedOutputShared(output);
}

/**
 * Rough chars→tokens estimate (~4 chars/token for typical English text +
 * code). The prune pass is a display/persistence optimization — exact counts
 * would require a tokenizer (e.g. tiktoken) heavier than this feature needs,
 * and the `tokenlens` dependency is cost/catalog-focused, not a tokenizer.
 * The budget is advisory; a 25% error margin doesn't change the UX outcome.
 */
export function estimateTokens(s: string): number {
  return estimateTokensShared(s);
}

/**
 * Walk `messages` and replace the `output` of completed tool-output parts
 * whose content falls outside the trailing `recencyTokens` budget with a
 * `{ cleared: true }` marker. Tool-call inputs and the most recent turns are
 * kept verbatim. Pure function — no side effects on the input array.
 *
 * The budget counts tool-output tokens from the END of the thread backwards
 * (the recency window is a trailing window). When the budget is exhausted,
 * every older tool output is cleared. Non-tool parts don't consume the
 * budget — only completed tool outputs do.
 */
export function pruneOldToolOutputs(
  messages: UIMessage[],
  recencyTokens: number,
): UIMessage[] {
  return pruneOldToolOutputsShared(messages, recencyTokens);
}
