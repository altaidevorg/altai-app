/**
 * Pure run-continue / terminal-attention copy (A6.138).
 * Hosts supply outcome shapes; no React and no bridge I/O.
 */

export type SharedRunBudgetSnapshot = {
  iterations_used: number;
  iterations_limit?: number;
  elapsed_ms?: number;
  elapsed_limit_ms?: number;
  tokens_used?: number;
  tokens_limit?: number;
  provider_retries_used?: number;
  provider_retries_limit?: number;
  context_recoveries_used?: number;
  context_recoveries_limit?: number;
  no_progress_turns?: number;
  repeated_root_cause_failures?: number;
  exhausted_limit?: string;
};

export type SharedRunBudgetWarning = {
  reason:
    | { kind: "approaching_limit"; limit: string }
    | { kind: "repeated_root_cause"; failures: number }
    | { kind: "no_progress"; turns: number };
  budget?: SharedRunBudgetSnapshot;
};

export type SharedRunOutcome =
  | { kind: "completed" }
  | { kind: "cancelled" }
  | { kind: "failed"; failure: string; retryable: boolean }
  | { kind: "stuck"; reason: string }
  | {
      kind: "budget_exhausted";
      budget: SharedRunBudgetSnapshot;
    };

/** Recoverable terminals — segment/stuck pauses, not provider crashes. */
export function isRecoverableRunOutcome(
  outcome: SharedRunOutcome | { kind: string } | null | undefined,
): boolean {
  return outcome?.kind === "stuck" || outcome?.kind === "budget_exhausted";
}

/**
 * User-facing attention string for a terminal outcome, or null when the run
 * ended cleanly. Stuck and budget exhaustion are framed as pauses.
 */
export function describeTerminalOutcomeAttention(
  outcome: SharedRunOutcome | null | undefined,
): string | null {
  if (!outcome || outcome.kind === "completed" || outcome.kind === "cancelled") {
    return null;
  }
  if (outcome.kind === "failed") return outcome.failure;
  if (outcome.kind === "stuck") {
    return `Run paused — ${outcome.reason.replace(/^Stopped:\s*/i, "")}`;
  }
  if (outcome.kind === "budget_exhausted") {
    return `Run paused — Hit the turn limit after ${outcome.budget.iterations_used} steps`;
  }
  return null;
}

export function continueStuckPrompt(): string {
  return "Continue the previous task from where it stopped. Reuse the existing context, avoid repeating successful side effects, and make measurable progress before completing.";
}

export function continueBudgetSegmentPrompt(): string {
  return "Continue the previous task from where it stopped. You have additional turns available now — pick up the unfinished work, reuse the existing context, avoid repeating successful side effects, and make measurable progress before completing.";
}

export function describeRunWarning(warning: SharedRunBudgetWarning): string {
  switch (warning.reason.kind) {
    case "approaching_limit":
      return `Run is approaching its ${warning.reason.limit.replace(/_/g, " ")} limit`;
    case "repeated_root_cause":
      return `The same typed failure repeated ${warning.reason.failures} times`;
    case "no_progress":
      return `No measurable progress for ${warning.reason.turns} turns`;
  }
}
