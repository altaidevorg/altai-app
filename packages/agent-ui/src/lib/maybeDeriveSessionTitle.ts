/**
 * Pure auto-title update decision for untitled sessions (A6.196).
 */

import { isUntitledSessionTitle } from "./isUntitledSessionTitle.js";
import { renameSessionInList } from "./renameSessionInList.js";

export type DerivableSession = { id: string; title: string; updatedAt: number };

/**
 * When a session is still untitled and `nextTitle` differs, return a renamed
 * list; otherwise `null` so the host can skip store writes.
 */
export function maybeDeriveSessionTitleList<T extends DerivableSession>(
  sessions: readonly T[],
  id: string,
  nextTitle: string,
  now: number = Date.now(),
): T[] | null {
  const meta = sessions.find((s) => s.id === id);
  if (!meta) return null;
  if (!isUntitledSessionTitle(meta.title)) return null;
  if (nextTitle === meta.title) return null;
  return renameSessionInList(sessions, id, nextTitle, now);
}
