/**
 * Revision for optimistic concurrency control.
 *
 * Every mutable aggregate carries a `Revision` that increases monotonically
 * on each accepted mutation. Concurrent edits must supply the expected
 * revision; a stale revision is rejected with `ControlError.staleRevision`.
 */

export type Revision = number;

export const INITIAL_REVISION: Revision = 0;

export function createRevision(value: number): Revision {
  return value;
}

export function nextRevision(rev: Revision): Revision {
  return rev + 1;
}

export function isRevision(value: unknown): value is Revision {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}
