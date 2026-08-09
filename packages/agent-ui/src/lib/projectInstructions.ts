/**
 * Pure project instruction path + combine helpers (A6.150).
 * Hosts still own filesystem I/O for reading ALTAI.md.
 */

export const PROJECT_INSTRUCTIONS_FILE = "ALTAI.md";
export const MAX_PROJECT_INSTRUCTIONS_CHARS = 16_000;

export function projectInstructionsPath(workspacePath: string): string {
  return `${workspacePath.replace(/[\\/]+$/, "")}/${PROJECT_INSTRUCTIONS_FILE}`;
}

/**
 * IsanAgent receives one persona string at runtime creation. Project rules
 * go ahead of the selected persona so both survive per-session isolation.
 */
export function combineAgentInstructions(
  agentInstructions: string | undefined,
  projectInstructions: string | undefined,
  fileName: string = PROJECT_INSTRUCTIONS_FILE,
): string | undefined {
  const sections = [
    projectInstructions
      ? `<project-instructions source="${fileName}">\n${projectInstructions}\n</project-instructions>`
      : "",
    agentInstructions ?? "",
  ].filter(Boolean);
  return sections.length ? sections.join("\n\n") : undefined;
}

/** Clamp project contract text after a host read. */
export function clampProjectInstructions(
  text: string,
  maxChars: number = MAX_PROJECT_INSTRUCTIONS_CHARS,
): string | undefined {
  const trimmed = text.trim();
  if (!trimmed) return undefined;
  return trimmed.slice(0, maxChars);
}
