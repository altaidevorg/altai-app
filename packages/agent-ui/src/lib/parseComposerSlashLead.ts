/**
 * Pure composer slash/hash lead parse (A6.173).
 * Host resolves the head name against its command index.
 */

export type ComposerSlashLead = {
  /** Original lead character, `/` or `#`. */
  lead: "/" | "#";
  /** First token after the lead (command name / alias). */
  head: string;
  /** Remainder of the input after the head token. */
  tail: string;
};

/**
 * Parse a composer line that starts with `/` or `#` into lead/head/tail.
 * Returns null when the line is not a slash-style command lead.
 */
export function parseComposerSlashLead(
  input: string,
): ComposerSlashLead | null {
  const trimmed = input.trim();
  const lead = trimmed[0];
  if (lead !== "/" && lead !== "#") return null;
  const [head = "", ...rest] = trimmed.slice(1).split(/\s+/);
  if (!head) return null;
  return {
    lead,
    head,
    tail: rest.join(" ").trim(),
  };
}
