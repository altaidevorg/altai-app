/**
 * Pure unknown→message and mutation key helpers (A6.205).
 */

/** Coerce an unknown throw value to a user-facing string. */
export function errorMessageFromUnknown(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Namespace a mutation id as `kind:id` for pending maps. */
export function mutationKey(kind: string, id: string): string {
  return `${kind}:${id}`;
}

/** Trim empty workspace paths to null. */
export function normalizedWorkspacePath(
  path?: string | null,
): string | null {
  const trimmed = path?.trim();
  return trimmed ? trimmed : null;
}
