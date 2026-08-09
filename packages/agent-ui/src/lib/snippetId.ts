/**
 * Pure snippet id generator (A6.153).
 */

export function newSnippetId(
  now: () => number = Date.now,
  random: () => number = Math.random,
): string {
  return `sn-${now().toString(36)}-${random().toString(36).slice(2, 6)}`;
}
