import type { WorkAttemptPhase, WorkRun, WorkState } from "@altai/host-contract";

/**
 * Runs hub projection (package 063, PR 2). One server projection
 * (`work_runs` — attempts joined with their Work) becomes the hub's rows.
 * Package 062's gate carries: the Work's status and the attempt's
 * execution phase remain distinct typed fields, never one label.
 */

export type WorkRunRow = {
  id: string;
  workId: string;
  workTitle: string;
  attemptLabel: string;
  status: WorkState;
  statusLabel: string;
  phase: WorkAttemptPhase;
  phaseLabel: string;
  updatedMs: number;
};

function label(value: string): string {
  return value.replace(/_/g, " ");
}

export function toWorkRunRow(run: WorkRun): WorkRunRow {
  return {
    id: run.id,
    workId: run.workId,
    workTitle: run.workTitle,
    attemptLabel: `Attempt ${run.number}`,
    status: run.workState,
    statusLabel: label(run.workState),
    phase: run.phase,
    phaseLabel: label(run.phase),
    updatedMs: run.updatedAtMs,
  };
}

/** Project the hub's runs, newest first. The server already orders the
 *  page; the projection preserves that order. */
export function projectRunsHub(runs: readonly WorkRun[]): WorkRunRow[] {
  return runs.map(toWorkRunRow);
}
