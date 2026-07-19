/**
 * Display/persistence-only prune for the chat transcript.
 *
 * Mirrors the Kilo "prune old tool results" behavior: completed tool outputs
 * that fall outside a trailing recency-token budget are collapsed to a small
 * `{ cleared: true }` marker in the persisted thread. The model's own context
 * is the runtime's responsibility (its native compaction already prunes) —
 * this is purely a DOM/disk footprint optimization so a long chat doesn't
 * bloat `altai-ai-sessions.json` and the rendered transcript.
 *
 * Pure functions only — no store side effects. Tests live alongside.
 */
import type { UIMessage } from "ai";

type AnyPart = UIMessage["parts"][number];

/** Marker text rendered in place of a cleared tool output. */
export const CLEARED_TOOL_OUTPUT_TEXT = "[Old tool result content cleared]";

/** Marker shape stored in the persisted part's `output` field. */
export const CLEARED_OUTPUT: { cleared: true } = { cleared: true };

/** True when a part's output has already been cleared by a prior prune pass. */
export function isClearedOutput(output: unknown): boolean {
  return (
    typeof output === "object" &&
    output !== null &&
    (output as { cleared?: unknown }).cleared === true
  );
}

/**
 * Rough chars→tokens estimate (~4 chars/token for typical English text +
 * code). The prune pass is a display/persistence optimization — exact counts
 * would require a tokenizer (e.g. tiktoken) heavier than this feature needs,
 * and the `tokenlens` dependency is cost/catalog-focused, not a tokenizer.
 * The budget is advisory; a 25% error margin doesn't change the UX outcome.
 */
export function estimateTokens(s: string): number {
  return Math.ceil(s.length / 4);
}

/** Coerce a tool part's output to a string for token estimation. Returns
 *  `null` when the part isn't a completed tool output worth pruning. */
function toolOutputString(part: AnyPart): string | null {
  if (typeof part !== "object" || part === null) return null;
  const p = part as { type?: string; output?: unknown; state?: string };
  const t = p.type ?? "";
  if (t !== "dynamic-tool" && !t.startsWith("tool-")) return null;
  // Only completed (output-available) parts are candidates. Errors and
  // pending calls retain their original shape so retries / debugging work.
  if (p.state !== "output-available") return null;
  const out = p.output;
  if (out == null) return null;
  if (isClearedOutput(out)) return null; // already pruned — don't double-count
  if (typeof out === "string") return out;
  try {
    return JSON.stringify(out);
  } catch {
    return String(out);
  }
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
  if (messages.length === 0) return messages;
  if (!Number.isFinite(recencyTokens) || recencyTokens <= 0) return messages;

  type Loc = { m: number; p: number; tokens: number };
  const locs: Loc[] = [];
  for (let m = 0; m < messages.length; m++) {
    const parts = messages[m].parts;
    for (let p = 0; p < parts.length; p++) {
      const text = toolOutputString(parts[p]);
      if (text == null) continue;
      locs.push({ m, p, tokens: estimateTokens(text) });
    }
  }
  if (locs.length === 0) return messages;

  // Walk newest-first: spend the budget on recent outputs. Anything we can't
  // afford (everything older than the trailing window) gets cleared.
  let budget = recencyTokens;
  const clear = new Set<string>();
  for (let i = locs.length - 1; i >= 0; i--) {
    const loc = locs[i];
    if (budget >= loc.tokens) {
      budget -= loc.tokens;
    } else {
      clear.add(`${loc.m}:${loc.p}`);
    }
  }
  if (clear.size === 0) return messages;

  let touched = false;
  const next = messages.map((msg, mi) => {
    let msgTouched = false;
    const parts = msg.parts.map((part, pi) => {
      if (!clear.has(`${mi}:${pi}`)) return part;
      if (toolOutputString(part) == null) return part; // idempotent re-check
      msgTouched = true;
      return {
        ...(part as object),
        output: CLEARED_OUTPUT,
      } as UIMessage["parts"][number];
    });
    if (!msgTouched) return msg;
    touched = true;
    return { ...msg, parts };
  });
  return touched ? next : messages;
}
