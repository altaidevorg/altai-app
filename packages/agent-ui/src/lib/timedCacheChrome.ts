/**
 * Pure TTL cache freshness + task title from prompt (A6.221).
 */

/** True when `nowMs - fetchedAt` is strictly less than `ttlMs`. */
export function isTimedCacheFresh(
  fetchedAt: number,
  ttlMs: number,
  nowMs: number = Date.now(),
): boolean {
  return nowMs - fetchedAt < ttlMs;
}

/** First non-empty line of a task prompt for list/title display. */
export function taskTitleFromPrompt(prompt: string): string {
  return prompt.trim().split("\n")[0] ?? "";
}
