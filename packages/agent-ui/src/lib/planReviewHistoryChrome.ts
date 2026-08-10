/**
 * Pure Plan Diff Review history list projection (A6.243).
 */

export type AppliedPlanEditLike = {
  id: string;
  path: string;
  isNewFile: boolean;
};

export type CheckpointHistoryLike = {
  id: string;
  path: string;
  label: string;
  createdMs: number;
};

export type PlanReviewHistoryRow = {
  id: string;
  path: string;
  detail: string;
};

/** Newest-first accepted plan-edit rows. */
export function planAppliedHistoryRows(
  applied: readonly AppliedPlanEditLike[],
): PlanReviewHistoryRow[] {
  return [...applied].reverse().map((item) => ({
    id: `plan-${item.id}`,
    path: item.path,
    detail: `Accepted review · ${
      item.isNewFile ? "remove new file" : "restore prior content"
    }`,
  }));
}

/** Checkpoint rows with host-formatted times. */
export function checkpointHistoryRows(
  items: readonly CheckpointHistoryLike[],
  formatTime: (createdMs: number) => string,
): PlanReviewHistoryRow[] {
  return items.map((item) => ({
    id: item.id,
    path: item.path,
    detail: `${item.label} · ${formatTime(item.createdMs)}`,
  }));
}

/** Combined review history (applied first, then checkpoints). */
export function composePlanReviewHistoryRows(
  applied: readonly AppliedPlanEditLike[],
  checkpoints: readonly CheckpointHistoryLike[],
  formatTime: (createdMs: number) => string,
): PlanReviewHistoryRow[] {
  return [
    ...planAppliedHistoryRows(applied),
    ...checkpointHistoryRows(checkpoints, formatTime),
  ];
}

export function isPlanRestoreRowId(rowId: string): boolean {
  return rowId.startsWith("plan-");
}

export function planIdFromRestoreRowId(rowId: string): string {
  return rowId.slice("plan-".length);
}
