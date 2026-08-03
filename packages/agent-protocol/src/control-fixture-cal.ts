export const WORK_ITEM_ID_TYPE = "work_item_id";
export const WORK_ITEM_ID_PREFIX = "wi_";

export type WorkItemId = {
  type: typeof WORK_ITEM_ID_TYPE;
  value: string;
};

export class WorkItemIdError extends Error {
  constructor(
    public readonly kind: "invalid_shape" | "wrong_type" | "empty_value" | "missing_prefix",
    public readonly value?: unknown,
  ) {
    super(kind);
  }
}

/** Parses an unknown value into a WorkItemId, rejecting malformed values with a typed WorkItemIdError. */
export function parseWorkItemId(input: unknown): WorkItemId {
  if (typeof input !== "object" || input === null || Array.isArray(input)) {
    throw new WorkItemIdError("invalid_shape", input);
  }
  const candidate = input as { type?: unknown; value?: unknown };
  if (candidate.type !== WORK_ITEM_ID_TYPE) throw new WorkItemIdError("wrong_type", candidate.type);
  if (typeof candidate.value !== "string") throw new WorkItemIdError("invalid_shape", candidate.value);
  if (candidate.value.length === 0) throw new WorkItemIdError("empty_value");
  if (!candidate.value.startsWith(WORK_ITEM_ID_PREFIX)) throw new WorkItemIdError("missing_prefix", candidate.value);
  return { type: WORK_ITEM_ID_TYPE, value: candidate.value };
}

/** Serializes to the canonical compact form shared byte-identically with the Rust implementation. */
export function serializeWorkItemId(id: WorkItemId): string {
  return JSON.stringify(id);
}
