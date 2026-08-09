/**
 * Pure transcript prune: clear older completed tool outputs outside a trailing
 * recency-token budget (A6.154). Display/persistence footprint only.
 */

import {
  CLEARED_OUTPUT,
  estimateTokens,
  isClearedOutput,
} from "./tokenEstimate.js";

/** Minimal message shape: role/parts transcript row. */
export type PruneableMessage = {
  parts: readonly unknown[];
};

function toolOutputString(part: unknown): string | null {
  if (typeof part !== "object" || part === null) return null;
  const p = part as { type?: string; output?: unknown; state?: string };
  const t = p.type ?? "";
  if (t !== "dynamic-tool" && !t.startsWith("tool-")) return null;
  // Only completed (output-available) parts are candidates.
  if (p.state !== "output-available") return null;
  const out = p.output;
  if (out == null) return null;
  if (isClearedOutput(out)) return null;
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
 * `{ cleared: true }` marker. Pure — does not mutate input.
 */
export function pruneOldToolOutputs<T extends PruneableMessage>(
  messages: T[],
  recencyTokens: number,
): T[] {
  if (messages.length === 0) return messages;
  if (!Number.isFinite(recencyTokens) || recencyTokens <= 0) return messages;

  type Loc = { m: number; p: number; tokens: number };
  const locs: Loc[] = [];
  for (let m = 0; m < messages.length; m++) {
    const parts = messages[m]!.parts;
    for (let p = 0; p < parts.length; p++) {
      const text = toolOutputString(parts[p]);
      if (text == null) continue;
      locs.push({ m, p, tokens: estimateTokens(text) });
    }
  }
  if (locs.length === 0) return messages;

  let budget = recencyTokens;
  const clear = new Set<string>();
  for (let i = locs.length - 1; i >= 0; i--) {
    const loc = locs[i]!;
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
      if (toolOutputString(part) == null) return part;
      msgTouched = true;
      return {
        ...(part as object),
        output: CLEARED_OUTPUT,
      };
    });
    if (!msgTouched) return msg;
    touched = true;
    return { ...msg, parts };
  });
  return touched ? next : messages;
}
