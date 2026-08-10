/**
 * Pure RunRecoveryActions title/detail/steer copy (A6.251).
 */

export type RunRecoveryOutcomeLike = {
  kind: string;
  reason?: string;
  retryable?: boolean;
  budget?: { iterations_used?: number };
};

export type RunRecoveryPresentation = {
  title: string;
  detail: string;
};

/**
 * Banner title for live warning, retryable failure, or recoverable pause.
 */
export function runRecoveryTitle(input: {
  hasWarning: boolean;
  canRetry: boolean;
  outcome?: RunRecoveryOutcomeLike | null;
}): string {
  if (input.hasWarning) return "Possible repeated failure";
  if (input.canRetry) return "Retry available";
  if (input.outcome?.kind === "budget_exhausted") return "Turn limit reached";
  return "Run paused";
}

/**
 * Banner detail. Host passes `describeRunWarning` text when a live warning exists.
 */
export function runRecoveryDetail(input: {
  warningDescription?: string | null;
  outcome?: RunRecoveryOutcomeLike | null;
}): string {
  if (input.warningDescription) {
    return `${input.warningDescription}. You can steer, stop, or dismiss — the run is still working.`;
  }
  if (input.outcome?.kind === "stuck") {
    const reason = (input.outcome.reason ?? "paused").replace(/_/g, " ");
    return `The run paused because it was ${reason}.`;
  }
  if (input.outcome?.kind === "budget_exhausted") {
    const steps = input.outcome.budget?.iterations_used ?? 0;
    return `Hit the turn limit after ${steps} steps. Continue picks up where it left off.`;
  }
  return "The provider request failed after its retry policy was exhausted.";
}

export function runRecoveryPresentation(input: {
  warningDescription?: string | null;
  canRetry: boolean;
  outcome?: RunRecoveryOutcomeLike | null;
}): RunRecoveryPresentation {
  return {
    title: runRecoveryTitle({
      hasWarning: Boolean(input.warningDescription),
      canRetry: input.canRetry,
      outcome: input.outcome,
    }),
    detail: runRecoveryDetail({
      warningDescription: input.warningDescription,
      outcome: input.outcome,
    }),
  };
}

/** Prefill when the host focuses the composer to steer. */
export function runRecoverySteerPrompt(hasWarning: boolean): string {
  return hasWarning
    ? "Adjust the active run with this direction: "
    : "Continue the previous run with this adjustment: ";
}
