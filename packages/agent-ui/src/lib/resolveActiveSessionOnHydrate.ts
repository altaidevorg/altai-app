/**
 * Pure bootstrap session selection for chat hydration (A6.191).
 */

import { isUntitledSessionTitle } from "./isUntitledSessionTitle.js";
import { newUntitledSessionMeta } from "./newSessionMeta.js";

export type SessionIdTitle = { id: string; title: string };

/**
 * Prefer persisted activeId, else first untitled session, else mint a fresh
 * untitled row. Does not mutate `sessions`.
 */
export function resolveActiveSessionOnHydrate<T extends SessionIdTitle>(
  sessions: readonly T[],
  activeId: string | null | undefined,
  createUntitled: () => T,
): { active: T; nextSessions: T[]; created: boolean } {
  let active =
    (activeId ? sessions.find((s) => s.id === activeId) : undefined) ?? null;
  if (!active && sessions[0] && isUntitledSessionTitle(sessions[0].title)) {
    active = sessions[0];
  }
  if (active) {
    return { active, nextSessions: [...sessions], created: false };
  }
  const created = createUntitled();
  return {
    active: created,
    nextSessions: [created, ...sessions],
    created: true,
  };
}

/** Convenience: mint untitled meta with shared title factory. */
export function createUntitledSessionMeta(id: string, now?: number) {
  return newUntitledSessionMeta({ id, now });
}
