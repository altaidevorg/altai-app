/**
 * Pure backend→frontend session list recovery (A6.167).
 * Host fetches backend rows; package merges without I/O.
 */

import { backendSessionTitle } from "./backendSessionTitle.js";

export type RecoverableSessionMeta = {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
};

export type BackendSessionRow = {
  id: string;
  updatedAt: number;
  title?: string | null;
};

/**
 * Merge backend-only sessions into a frontend list.
 * Skips known ids and a permanent-delete blocklist.
 * Does not mutate `frontend`; returns a new sorted array.
 */
export function mergeRecoveredSessions<T extends RecoverableSessionMeta>(
  frontend: readonly T[],
  backend: readonly BackendSessionRow[],
  deletedIds: readonly string[] = [],
): { merged: Array<T | RecoverableSessionMeta>; recoveredIds: string[] } {
  const known = new Set(frontend.map((s) => s.id));
  const deleted = new Set(deletedIds);
  const recoveredIds: string[] = [];
  const additions: RecoverableSessionMeta[] = [];

  for (const b of backend) {
    if (known.has(b.id)) continue;
    if (deleted.has(b.id)) continue;
    additions.push({
      id: b.id,
      title: backendSessionTitle(b.title),
      createdAt: b.updatedAt,
      updatedAt: b.updatedAt,
    });
    known.add(b.id);
    recoveredIds.push(b.id);
  }

  const merged: Array<T | RecoverableSessionMeta> = [
    ...frontend,
    ...additions,
  ];
  merged.sort((a, b) => b.updatedAt - a.updatedAt);
  return { merged, recoveredIds };
}
