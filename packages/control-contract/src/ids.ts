/**
 * Canonical typed IDs for every control-plane concept (parent plan §3.3).
 *
 * Each ID serializes as `{"type":"<kind>","value":"<prefix>..."}`. The
 * `type` field makes the JSON self-describing and prevents accidental
 * substitution of one ID for another. Parsing rejects:
 *
 * - non-object input,
 * - wrong or missing `type` field,
 * - empty value,
 * - value missing the required prefix.
 */

export type IdKind =
  | "organization_id"
  | "goal_id"
  | "project_id"
  | "workspace_id"
  | "agent_instance_id"
  | "agent_profile_id"
  | "agent_profile_revision_id"
  | "work_item_id"
  | "attempt_id"
  | "run_id"
  | "session_id"
  | "routine_id"
  | "routine_revision_id"
  | "routine_run_id"
  | "approval_id"
  | "external_object_id";

export type TypedId = {
  readonly type: IdKind;
  readonly value: string;
};

export type IdErrorKind =
  | "invalid_shape"
  | "missing_type"
  | "wrong_type"
  | "empty_value"
  | "missing_prefix";

export class IdError extends Error {
  constructor(
    public readonly kind: IdErrorKind,
    public readonly detail?: unknown,
  public readonly expected?: string,
    public readonly got?: string,
  ) {
    super(kind);
    this.name = "IdError";
  }
}

interface IdSpec {
  readonly type: IdKind;
  readonly prefix: string;
}

const ID_SPECS: Record<IdKind, IdSpec> = {
  organization_id: { type: "organization_id", prefix: "org_" },
  goal_id: { type: "goal_id", prefix: "goal_" },
  project_id: { type: "project_id", prefix: "proj_" },
  workspace_id: { type: "workspace_id", prefix: "ws_" },
  agent_instance_id: { type: "agent_instance_id", prefix: "ai_" },
  agent_profile_id: { type: "agent_profile_id", prefix: "ap_" },
  agent_profile_revision_id: { type: "agent_profile_revision_id", prefix: "apr_" },
  work_item_id: { type: "work_item_id", prefix: "wi_" },
  attempt_id: { type: "attempt_id", prefix: "att_" },
  run_id: { type: "run_id", prefix: "run_" },
  session_id: { type: "session_id", prefix: "sess_" },
  routine_id: { type: "routine_id", prefix: "rt_" },
  routine_revision_id: { type: "routine_revision_id", prefix: "rtr_" },
  routine_run_id: { type: "routine_run_id", prefix: "rr_" },
  approval_id: { type: "approval_id", prefix: "apv_" },
  external_object_id: { type: "external_object_id", prefix: "ext_" },
};

function defineId<K extends IdKind>(kind: K) {
  const spec = ID_SPECS[kind];
  return {
    TYPE: spec.type,
    PREFIX: spec.prefix,
    /** Create a new ID from a raw value string, prepending the prefix if missing. */
    create(value: string): TypedId {
      const v = value.startsWith(spec.prefix) ? value : `${spec.prefix}${value}`;
      return { type: spec.type, value: v };
    },
    /** Parse an unknown value into a TypedId, rejecting malformed input with a typed IdError. */
    parse(input: unknown): TypedId {
      if (typeof input !== "object" || input === null || Array.isArray(input)) {
        throw new IdError("invalid_shape", input);
      }
      const candidate = input as { type?: unknown; value?: unknown };
      if (candidate.type === undefined) throw new IdError("missing_type");
      if (candidate.type !== spec.type) {
        throw new IdError("wrong_type", candidate.type, spec.type, String(candidate.type));
      }
      if (typeof candidate.value !== "string") {
        throw new IdError("invalid_shape", candidate.value);
      }
      if (candidate.value.length === 0) throw new IdError("empty_value");
      if (!candidate.value.startsWith(spec.prefix)) {
        throw new IdError("missing_prefix", candidate.value, spec.prefix, candidate.value);
      }
      return { type: spec.type, value: candidate.value };
    },
    /** Type guard: true if the value is a valid TypedId of this kind. */
    is(input: unknown): input is TypedId {
      try {
        return this.parse(input).type === spec.type;
      } catch {
        return false;
      }
    },
  };
}

export const OrganizationId = defineId("organization_id");
export const GoalId = defineId("goal_id");
export const ProjectId = defineId("project_id");
export const WorkspaceId = defineId("workspace_id");
export const AgentInstanceId = defineId("agent_instance_id");
export const AgentProfileId = defineId("agent_profile_id");
export const AgentProfileRevisionId = defineId("agent_profile_revision_id");
export const WorkItemId = defineId("work_item_id");
export const AttemptId = defineId("attempt_id");
export const RunId = defineId("run_id");
export const SessionId = defineId("session_id");
export const RoutineId = defineId("routine_id");
export const RoutineRevisionId = defineId("routine_revision_id");
export const RoutineRunId = defineId("routine_run_id");
export const ApprovalId = defineId("approval_id");
export const ExternalObjectId = defineId("external_object_id");

/** Serialize a TypedId to the canonical compact JSON form. */
export function serializeId(id: TypedId): string {
  return JSON.stringify(id);
}

/** All ID kinds, for iteration and validation. */
export const ALL_ID_KINDS: readonly IdKind[] = Object.keys(ID_SPECS) as IdKind[];
