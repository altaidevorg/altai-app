/**
 * Desktop plan-review queue. Disk mutates only through `planEditFs` so
 * ReviewPort and this store share one apply/restore path (Wave 1.3).
 */

import { create } from "zustand";
import { native } from "../lib/native";
import {
  editProposalInputFromQueued as editProposalInputFromQueuedShared,
  markPlanEditAppliedState,
} from "@altai/agent-ui";
import {
  applyPlanEditMutation,
  restorePlanEditMutation,
  type PlanEditFs,
} from "../lib/planEditFs";

export type QueuedEdit = {
  id: string;
  /** Tool that produced the queued mutation. */
  kind: "write_file" | "edit" | "multi_edit" | "create_directory";
  path: string;
  /** Original file content (empty for new files / create_directory). */
  originalContent: string;
  /** Proposed full content after edit (empty for create_directory). */
  proposedContent: string;
  /** True if the file did not exist when the edit was queued. */
  isNewFile: boolean;
  /** Human-readable description, used for create_directory. */
  description?: string;
};

/** A locally reversible change accepted from Plan review in this app session. */
export type AppliedPlanEdit = QueuedEdit & {
  appliedAt: number;
};

export type PlanApplyResult = { id: string; ok: boolean; error?: string };

const defaultFs: PlanEditFs = {
  writeFile: (path, content, opts) =>
    native.writeFile(path, content, {
      source: opts?.source ?? "ai-plan-review",
    }),
  createDir: (path) => native.createDir(path),
  delete: (path) => native.delete(path),
};

/** Test/host seam: override FS used by plan apply/restore. */
let planFs: PlanEditFs = defaultFs;

export function setPlanEditFs(next: PlanEditFs | null): void {
  planFs = next ?? defaultFs;
}

export function editProposalInputFromQueued(item: QueuedEdit) {
  return editProposalInputFromQueuedShared(item) as {
    path: string;
    kind:
      | "edit_file"
      | "create_file"
      | "create_directory"
      | "write_file"
      | "edit"
      | "multi_edit";
    originalContent: string;
    proposedContent: string;
  };
}

type PlanState = {
  active: boolean;
  queue: QueuedEdit[];
  /** Reversible plan-review edits. Directory creates are excluded: they cannot be safely removed once populated. */
  applied: AppliedPlanEdit[];
  toggle: () => void;
  enable: () => void;
  disable: () => void;
  enqueue: (q: QueuedEdit) => void;
  removeOne: (id: string) => void;
  clear: () => void;
  /**
   * Record a successful external apply (e.g. ReviewPort) without writing again.
   */
  recordApplied: (id: string) => PlanApplyResult | null;
  /** Apply exactly one reviewed edit via planEditFs and keep a local rollback snapshot when safe. */
  applyOne: (id: string) => Promise<PlanApplyResult | null>;
  /** Apply queued edits in order. Returns per-edit results. */
  applyAll: () => Promise<PlanApplyResult[]>;
  /** Restore the pre-review content for one locally applied edit. */
  restoreApplied: (id: string) => Promise<PlanApplyResult | null>;
};

function markAppliedState(
  queue: QueuedEdit[],
  applied: AppliedPlanEdit[],
  item: QueuedEdit,
): { queue: QueuedEdit[]; applied: AppliedPlanEdit[] } {
  return markPlanEditAppliedState(queue, applied, item) as {
    queue: QueuedEdit[];
    applied: AppliedPlanEdit[];
  };
}

export const usePlanStore = create<PlanState>((set, get) => ({
  active: false,
  queue: [],
  applied: [],
  toggle: () =>
    set((s) => ({ active: !s.active, queue: s.active ? [] : s.queue })),
  enable: () => set({ active: true }),
  disable: () => set({ active: false, queue: [] }),
  enqueue: (q) => set((s) => ({ queue: [...s.queue, q] })),
  removeOne: (id) =>
    set((s) => ({ queue: s.queue.filter((q) => q.id !== id) })),
  clear: () => set({ queue: [] }),
  recordApplied(id) {
    const item = get().queue.find((q) => q.id === id);
    if (!item) return null;
    set((s) => markAppliedState(s.queue, s.applied, item));
    return { id, ok: true };
  },
  async applyOne(id) {
    const item = get().queue.find((q) => q.id === id);
    if (!item) return null;
    try {
      await applyPlanEditMutation(planFs, item, "ai-plan-review");
      set((s) => markAppliedState(s.queue, s.applied, item));
      return { id, ok: true };
    } catch (error) {
      return { id, ok: false, error: String(error) };
    }
  },
  async applyAll() {
    const ids = get().queue.map((q) => q.id);
    const results: PlanApplyResult[] = [];
    for (const nextId of ids) {
      const result = await get().applyOne(nextId);
      if (result) results.push(result);
    }
    return results;
  },
  async restoreApplied(id) {
    const item = get().applied.find((q) => q.id === id);
    if (!item) return null;
    try {
      await restorePlanEditMutation(planFs, item, "ai-plan-restore");
      set((s) => ({ applied: s.applied.filter((q) => q.id !== id) }));
      return { id, ok: true };
    } catch (error) {
      return { id, ok: false, error: String(error) };
    }
  },
}));
