/**
 * Pure Task Runs assignment status projection from live agent run (A6.213).
 */

export type TaskAssignmentStatus =
  | "done"
  | "failed"
  | "cancelled"
  | "running"
  | "awaiting-approval"
  | string;

export type TaskRunSnapshotLike = {
  completed?: boolean;
  status?: string | null;
  outcome?: { kind?: string | null } | null;
};

const DEFAULT_TERMINAL: readonly string[] = ["done", "failed", "cancelled"];

/**
 * Prefer terminal assignment status; otherwise project from the live run
 * (completed outcome / streaming / waiting on approval / error).
 */
export function resolveAssignmentStatusFromRun(
  assignmentStatus: string,
  run: TaskRunSnapshotLike | null | undefined,
  terminalStatuses: readonly string[] = DEFAULT_TERMINAL,
): string {
  if (terminalStatuses.includes(assignmentStatus) || !run) {
    return assignmentStatus;
  }
  if (run.completed) {
    if (run.outcome?.kind === "completed") return "done";
    if (run.outcome?.kind === "cancelled") return "cancelled";
    return "failed";
  }
  if (run.status === "thinking" || run.status === "streaming") return "running";
  if (run.status === "awaiting-approval") return "awaiting-approval";
  if (run.status === "error") return "failed";
  return assignmentStatus;
}
