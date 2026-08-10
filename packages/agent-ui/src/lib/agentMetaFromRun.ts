/**
 * Pure agent chrome projection from a completed/live run snapshot (A6.201).
 * Hosts own storage; package maps run status → inspector AgentMeta-ish fields.
 */

export type SharedRunTokenSnapshot = {
  input: number;
  output: number;
  cached: number;
};

export type SharedRunOutcomeLike =
  | { kind: "failed"; message?: string }
  | { kind: string; message?: string }
  | null
  | undefined;

export type SharedRunStateLike = {
  completed: boolean;
  status: string;
  step: string | null;
  outcome?: SharedRunOutcomeLike;
  tokens: SharedRunTokenSnapshot;
  subagents: unknown[];
};

export type ProjectedAgentRunStatus =
  | "idle"
  | "error"
  | string;

export type ProjectedAgentTokens = {
  inputTokens: number;
  outputTokens: number;
  cachedInputTokens: number;
};

/**
 * Map a run record into compact agent status fields used by host stores.
 * `describeError` is injected so hosts can reuse package attention copy.
 */
export function projectAgentMetaFromRun(
  run: SharedRunStateLike | null | undefined,
  describeError: (outcome: unknown) => string | null,
): {
  status: ProjectedAgentRunStatus;
  step: string | null;
  error: string | null;
  tokens: ProjectedAgentTokens;
  activeSubagents: unknown[];
} | null {
  if (!run) return null;
  const error = describeError(run.outcome);
  const status: ProjectedAgentRunStatus =
    run.completed && run.outcome?.kind === "failed"
      ? "error"
      : run.completed
        ? "idle"
        : run.status;
  return {
    status,
    step: run.completed ? null : run.step,
    error,
    tokens: {
      inputTokens: run.tokens.input,
      outputTokens: run.tokens.output,
      cachedInputTokens: run.tokens.cached,
    },
    activeSubagents: run.subagents,
  };
}
