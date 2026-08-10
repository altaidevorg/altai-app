/**
 * Pure Task Runs context-path + bot-title helpers (A6.225).
 */

/** Normalize Tauri/VS dialog selection (string | string[] | null). */
export function normalizeDialogPathSelection(
  selected: string | string[] | null | undefined,
): string[] {
  if (typeof selected === "string") {
    const path = selected.trim();
    return path ? [path] : [];
  }
  if (!Array.isArray(selected)) return [];
  return selected
    .filter((item): item is string => typeof item === "string")
    .map((item) => item.trim())
    .filter(Boolean);
}

/**
 * Append unique workspace paths to a context list, newest-last, capped.
 */
export function appendUniqueContextPaths(
  current: readonly string[],
  paths: readonly string[],
  max = 12,
): string[] {
  const next = [...current];
  const seen = new Set(current);
  for (const raw of paths) {
    const path = raw.trim();
    if (!path || seen.has(path)) continue;
    seen.add(path);
    next.push(path);
  }
  return next.slice(0, max);
}

/** Strip the leading agent emoji prefix used on stored task titles. */
export function stripTaskBotTitlePrefix(title: string): string {
  return title.replace(/^🤖\s*/, "");
}
