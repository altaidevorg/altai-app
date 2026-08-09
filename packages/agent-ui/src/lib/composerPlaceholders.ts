/**
 * Shared composer placeholder rotation copy (A6.146).
 */

export const COMPOSER_PLACEHOLDERS = [
  "Explain this error…",
  "Summarize the last command output",
  "Write a bash one-liner that…",
  "Refactor the selected code",
  "Generate a .gitignore for this project",
  "What does this stack trace mean?",
  "Draft a commit message for staged changes",
  "Find files larger than 50MB",
  "Convert this JSON to a TypeScript type",
  "Why is my build failing?",
] as const;

/**
 * Pick one placeholder. Inject `random` (0..1) for deterministic tests.
 * Defaults to Math.random.
 */
export function pickPlaceholder(
  random: () => number = Math.random,
  catalog: readonly string[] = COMPOSER_PLACEHOLDERS,
): string {
  if (catalog.length === 0) return "";
  const r = random();
  const idx = Math.min(
    catalog.length - 1,
    Math.max(0, Math.floor(r * catalog.length)),
  );
  return catalog[idx]!;
}
