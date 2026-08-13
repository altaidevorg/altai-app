import type { Revision } from "./revision.js";
import type { TypedId } from "./ids.js";

/** Global control-plane kinds; local Work cards must not invent more kinds. */
export type WorkItemKind = "task" | "ticket" | "campaign";

/** Human-facing global Work lifecycle. */
export type WorkStatus =
  | "backlog"
  | "todo"
  | "in_progress"
  | "in_review"
  | "done"
  | "blocked"
  | "cancelled";

/** Execution lifecycle, deliberately separate from the Work lifecycle. */
export type ExecutionPhase =
  | "none"
  | "queued"
  | "planning"
  | "awaiting_plan_approval"
  | "running"
  | "awaiting_input"
  | "awaiting_approval"
  | "verifying"
  | "reviewing"
  | "retrying"
  | "paused"
  | "failed"
  | "needs_attention"
  | "terminal";

/** Canonical global Work projection; parentage is not a dependency edge. */
export type ControlWorkItem = {
  readonly id: TypedId;
  readonly project_id: TypedId;
  readonly goal_id: TypedId | null;
  readonly parent_work_item_id: TypedId | null;
  readonly kind: WorkItemKind;
  readonly title: string;
  readonly description: string;
  readonly status: WorkStatus;
  readonly execution_phase: ExecutionPhase;
  readonly revision: Revision;
  readonly created_at: string;
  readonly updated_at: string;
};
