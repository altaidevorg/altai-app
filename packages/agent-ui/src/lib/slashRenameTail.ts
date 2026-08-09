/**
 * Pure slash rename/tail validation (A6.180).
 */

/** True when a slash command tail carries a non-empty title/arg. */
export function hasSlashCommandTail(tail: string): boolean {
  return tail.trim().length > 0;
}
