import type { Actor } from "./actor.js";
import type { Revision } from "./revision.js";
import type { TypedId } from "./ids.js";

/** Durable organization boundary for policy, identity, and budget. */
export type Organization = {
  readonly id: TypedId;
  readonly name: string;
  readonly revision: Revision;
  readonly created_at: string;
  readonly updated_at: string;
};

/** Goal ancestry is constrained to a single organization and is acyclic. */
export type Goal = {
  readonly id: TypedId;
  readonly organization_id: TypedId;
  readonly parent_goal_id: TypedId | null;
  readonly owner: Actor | null;
  readonly title: string;
  readonly description: string;
  readonly revision: Revision;
  readonly created_at: string;
  readonly updated_at: string;
};

export type ProjectStatus = "active" | "paused" | "archived";

/** An organization-scoped delivery context that can support several goals. */
export type Project = {
  readonly id: TypedId;
  readonly organization_id: TypedId;
  readonly goal_ids: readonly TypedId[];
  readonly name: string;
  readonly description: string;
  readonly status: ProjectStatus;
  readonly revision: Revision;
  readonly created_at: string;
  readonly updated_at: string;
};

/**
 * Durable project workspace identity. `local_path_hint` is mutable host
 * metadata, never its identity: moving a checkout must not make a new workspace.
 */
export type ProjectWorkspace = {
  readonly id: TypedId;
  readonly project_id: TypedId;
  readonly name: string;
  readonly repository_url: string | null;
  readonly local_path_hint: string | null;
  readonly revision: Revision;
  readonly created_at: string;
  readonly updated_at: string;
};
