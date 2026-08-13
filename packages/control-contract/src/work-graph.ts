import type { Actor } from "./actor.js";
import type { Revision } from "./revision.js";
import type { TypedId } from "./ids.js";

/** A blocker edge, intentionally separate from parent/sub-work structure. */
export type WorkDependency = { readonly work_item_id: TypedId; readonly blocker_work_item_id: TypedId; readonly created_at: string };
/** Durable actor-attributed work communication. */
export type WorkComment = {
  readonly id: string; readonly work_item_id: TypedId; readonly actor: Actor; readonly body: string;
  readonly created_by_attempt_id: TypedId | null; readonly created_by_run_id: TypedId | null;
  readonly revision: Revision; readonly created_at: string;
};
export type WorkRelationKind = "parent" | "blocks";
