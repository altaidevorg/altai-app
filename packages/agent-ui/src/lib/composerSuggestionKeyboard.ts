/**
 * Pure composer suggestion-list keyboard / open state (A6.65).
 * Shared by slash (`/`) and snippet (`#`) popovers.
 */

export type ComposerSuggestionKeyAction =
  | { type: "close" }
  | { type: "ignore" }
  | { type: "move"; index: number }
  | { type: "pick"; index: number };

/** Clamp / step active index within a suggestion list. */
export function nextSuggestionActiveIndex(
  current: number,
  length: number,
  direction: "up" | "down",
): number {
  if (length <= 0) return 0;
  if (direction === "down") {
    return Math.min(length - 1, current + 1);
  }
  return Math.max(0, current - 1);
}

/**
 * Whether a slash/snippet popover is open for a given prefix, and its query.
 */
export function resolveComposerSuggestionOpen(input: {
  trigger: { prefix: string; query: string } | null;
  forceClosed: boolean;
  prefix: "/" | "#";
}): { open: boolean; query: string } {
  const { trigger, forceClosed, prefix } = input;
  if (!trigger || forceClosed || trigger.prefix !== prefix) {
    return { open: false, query: "" };
  }
  return { open: true, query: trigger.query };
}

/**
 * Map arrow/enter/tab/escape while a suggestion list is focused.
 * Callers pass matchCount === 0 to avoid stealing Enter for composer submit.
 */
export function resolveComposerSuggestionKeyAction(
  key: string,
  state: { matchCount: number; activeIndex: number },
): ComposerSuggestionKeyAction {
  if (key === "Escape") {
    return { type: "close" };
  }
  if (state.matchCount === 0) {
    return { type: "ignore" };
  }
  if (key === "ArrowDown") {
    return {
      type: "move",
      index: nextSuggestionActiveIndex(
        state.activeIndex,
        state.matchCount,
        "down",
      ),
    };
  }
  if (key === "ArrowUp") {
    return {
      type: "move",
      index: nextSuggestionActiveIndex(
        state.activeIndex,
        state.matchCount,
        "up",
      ),
    };
  }
  if (key === "Enter" || key === "Tab") {
    const index = Math.max(
      0,
      Math.min(state.activeIndex, state.matchCount - 1),
    );
    return { type: "pick", index };
  }
  return { type: "ignore" };
}
