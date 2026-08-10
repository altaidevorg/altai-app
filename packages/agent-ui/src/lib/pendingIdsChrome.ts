/**
 * Pure pending-id map helpers for store mutation chrome (A6.207).
 */

export type PendingIdMap = Record<string, true>;

/** Mark a mutation key as in flight and clear surface error. */
export function withPendingStarted(
  pendingIds: PendingIdMap,
  key: string,
): { error: null; pendingIds: PendingIdMap } {
  return {
    error: null,
    pendingIds: { ...pendingIds, [key]: true },
  };
}

/**
 * Clear a mutation key when it is still present. Returns empty patch when
 * the key was already cleared (e.g. concurrent refresh reset).
 */
export function withPendingEnded(
  pendingIds: PendingIdMap,
  key: string,
): { pendingIds: PendingIdMap } | Record<string, never> {
  if (!pendingIds[key]) return {};
  const next: PendingIdMap = { ...pendingIds };
  delete next[key];
  return { pendingIds: next };
}
