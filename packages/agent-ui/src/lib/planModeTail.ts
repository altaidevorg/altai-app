/**
 * Pure plan-mode slash tail helpers (A6.175).
 */

/** True when `/plan` tail requests turning plan mode off. */
export function isPlanModeOffTail(tail: string): boolean {
  const t = tail.trim().toLowerCase();
  return t === "off" || t === "exit";
}
