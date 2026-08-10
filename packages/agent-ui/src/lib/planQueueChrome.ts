/**
 * Pure plan-review queue chrome (A6.204).
 */

import { proposalKindFromPlanEdit } from "./proposalKind.js";

export type QueuedPlanEditLike = {
  id: string;
  kind: string;
  path: string;
  originalContent: string;
  proposedContent: string;
  isNewFile: boolean;
  description?: string;
};

/** Map a queued plan edit to the ReviewPort proposal input shape. */
export function editProposalInputFromQueued(item: QueuedPlanEditLike) {
  return {
    path: item.path,
    kind: proposalKindFromPlanEdit(item.kind, item.isNewFile),
    originalContent: item.originalContent,
    proposedContent: item.proposedContent,
  };
}

/**
 * After a successful apply: drop from queue; keep applied snapshot unless
 * the edit is a directory create (unsafe to reverse blindly).
 */
export function markPlanEditAppliedState<T extends QueuedPlanEditLike>(
  queue: readonly T[],
  applied: readonly (T & { appliedAt: number })[],
  item: T,
  appliedAt: number = Date.now(),
): { queue: T[]; applied: Array<T & { appliedAt: number }> } {
  return {
    queue: queue.filter((q) => q.id !== item.id),
    applied:
      item.kind === "create_directory"
        ? [...applied]
        : [...applied, { ...item, appliedAt }],
  };
}
