/**
 * Pure Task Run card label / metric helpers (A6.229).
 */

/** Sum input+output token counters for run cards (0 when missing). */
export function sumRunTokens(
  tokens: { input: number; output: number } | null | undefined,
): number {
  if (!tokens) return 0;
  return tokens.input + tokens.output;
}

/** Comma-separated skill names for cards; undefined when empty. */
export function skillsListLabel(
  skills: readonly string[] | null | undefined,
): string | undefined {
  if (!skills?.length) return undefined;
  return skills.join(", ");
}

/**
 * Name for a catalog id (agents, etc.), or `fallback` when known-but-unnamed.
 * Returns undefined when no id is set.
 */
export function catalogEntryName(
  catalog: readonly { id: string; name?: string | null }[],
  id: string | null | undefined,
  fallback: string,
): string | undefined {
  if (!id) return undefined;
  const entry = catalog.find((item) => item.id === id);
  return entry?.name ?? fallback;
}
