/**
 * Pure session id generator (A6.151).
 * Inject now/random for tests.
 */

export function newSessionId(
  now: () => number = Date.now,
  random: () => number = Math.random,
): string {
  return `s-${now().toString(36)}-${random().toString(36).slice(2, 8)}`;
}
