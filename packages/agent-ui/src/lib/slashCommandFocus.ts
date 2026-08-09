/**
 * Pure slash-command prompt focus suffix (A6.172).
 * Host owns command prompt templates / content.
 */

/** Append an optional "Focus from the user" paragraph to a base slash prompt. */
export function appendSlashCommandFocus(
  basePrompt: string,
  tail: string,
): string {
  const trimmed = tail.trim();
  const focus = trimmed ? `

Focus from the user: ${trimmed}` : "";
  return `${basePrompt}${focus}`;
}
