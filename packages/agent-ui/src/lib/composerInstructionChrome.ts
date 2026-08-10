/**
 * Pure composer instruction prefix helper (A6.242).
 */

/**
 * Prepend a host instruction to the current draft. Empty drafts keep the
 * prefix with trailing blank lines so the caret lands ready to type.
 */
export function prependComposerInstruction(
  value: string,
  prefix: string,
): string {
  return value.trim() ? `${prefix}\n\n${value}` : `${prefix}\n\n`;
}

/** Default Semble Scout instruction used by the Desktop attach menu. */
export const SEMBLE_SCOUT_SEARCH_INSTRUCTION =
  "Use the Semble Scout subagent to search this workspace before answering.";
