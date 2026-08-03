/**
 * GLM-CAL-03: pure mapping from the legacy assignment record shape
 * (`TaskRunInfo` in `types.ts`) to the canonical WorkItem status axes defined
 * by the control-plane plan (Sections 5.1/5.2).
 *
 * The mapping is pure: no I/O, no network, no database access, deterministic.
 * Unknown legacy statuses are rejected with a typed error, never silently
 * mapped. Legacy IDs are preserved verbatim in `legacy_compat_id`; no durable
 * `work_item_id` is minted here — that is the CP-20 migration runner's job.
 */

/** Canonical work status values (control-plane plan, Section 5.1). */
export const WORK_STATUSES = [
  "backlog",
  "todo",
  "in_progress",
  "in_review",
  "blocked",
  "done",
  "cancelled",
] as const;
export type WorkStatus = (typeof WORK_STATUSES)[number];

/** Canonical execution phase values (control-plane plan, Section 5.2). */
export const EXECUTION_PHASES = [
  "none",
  "queued",
  "planning",
  "awaiting_plan_approval",
  "running",
  "awaiting_input",
  "awaiting_approval",
  "verifying",
  "reviewing",
  "retrying",
  "paused",
  "failed",
  "needs_attention",
  "terminal",
] as const;
export type ExecutionPhase = (typeof EXECUTION_PHASES)[number];

/** Canonical WorkItem draft produced by the mapping. Not a durable record:
 * `work_item_id` is always `null` because this pure function never invents
 * IDs — the CP-20 migration runner assigns durable IDs. */
export type CanonicalWorkItemDraft = {
  work_item_id: string | null;
  title: string;
  work_status: WorkStatus;
  execution_phase: ExecutionPhase;
  legacy_compat_id: string;
  created_at: string;
};

export type LegacyMappingErrorKind =
  | "missing_required_field"
  | "unknown_legacy_status"
  | "invalid_legacy_id";

/** Typed mapping error. Malformed input is always rejected, never silently mapped. */
export class LegacyMappingError extends Error {
  constructor(
    public readonly kind: LegacyMappingErrorKind,
    public readonly field?: string,
    public readonly value?: unknown,
  ) {
    super(field ? `${kind}: ${field}` : kind);
    this.name = "LegacyMappingError";
  }
}

/**
 * Status mapping (amended 2026-08-03): legacy `failed` maps to work_status
 * `in_progress` + execution_phase `failed` — the work was started but is not
 * complete; attention is derived from the execution phase, not WorkStatus.
 */
const STATUS_MAPPING: Record<string, { work_status: WorkStatus; execution_phase: ExecutionPhase }> = {
  queued: { work_status: "todo", execution_phase: "queued" },
  running: { work_status: "in_progress", execution_phase: "running" },
  succeeded: { work_status: "done", execution_phase: "terminal" },
  failed: { work_status: "in_progress", execution_phase: "failed" },
  cancelled: { work_status: "cancelled", execution_phase: "terminal" },
};

function requiredString(candidate: Record<string, unknown>, key: string, field: string): string {
  const value = candidate[key];
  if (typeof value !== "string") {
    throw new LegacyMappingError("missing_required_field", field, value);
  }
  return value;
}

/** Maps one legacy assignment record to the canonical WorkItem status axes.
 * Pure: same input always produces the same output, with no side effects. */
export function mapLegacyAssignment(input: unknown): CanonicalWorkItemDraft {
  if (typeof input !== "object" || input === null || Array.isArray(input)) {
    throw new LegacyMappingError("missing_required_field", "assignment", input);
  }
  const candidate = input as Record<string, unknown>;

  const idValue = candidate.id;
  if (idValue === null || idValue === undefined) {
    throw new LegacyMappingError("missing_required_field", "id");
  }
  if (typeof idValue !== "string" || idValue.length === 0) {
    throw new LegacyMappingError("invalid_legacy_id", "id", idValue);
  }

  const title = requiredString(candidate, "title", "title");
  const statusValue = candidate.status;
  if (statusValue === null || statusValue === undefined) {
    throw new LegacyMappingError("missing_required_field", "status");
  }
  if (typeof statusValue !== "string") {
    throw new LegacyMappingError("missing_required_field", "status", statusValue);
  }
  const createdAt = requiredString(candidate, "createdAt", "created_at");

  const mapped = STATUS_MAPPING[statusValue];
  if (!mapped) {
    throw new LegacyMappingError("unknown_legacy_status", "status", statusValue);
  }

  return {
    work_item_id: null,
    title,
    work_status: mapped.work_status,
    execution_phase: mapped.execution_phase,
    legacy_compat_id: idValue,
    created_at: createdAt,
  };
}
