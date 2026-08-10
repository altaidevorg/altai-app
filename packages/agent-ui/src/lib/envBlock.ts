/**
 * Pure <env> block builders for run payloads (A6.199).
 * Hosts supply live workspace/cwd/file facts; package formats the tag.
 */

export type LiveEnvFacts = {
  workspaceRoot?: string | null;
  cwd?: string | null;
  activeFile?: string | null;
  activeTerminalPrivate?: boolean;
};

/** Format key/value lines into an <env> … </env> block; null when empty. */
export function formatEnvBlock(lines: readonly string[]): string | null {
  if (lines.length === 0) return null;
  return `<env>\n${lines.join("\n")}\n</env>`;
}

/** Build live workspace/cwd/file env context for the next agent turn. */
export function buildEnvBlockFromFacts(facts: LiveEnvFacts): string | null {
  const lines: string[] = [];
  if (facts.workspaceRoot) lines.push(`workspace_root: ${facts.workspaceRoot}`);
  if (facts.cwd) lines.push(`active_terminal_cwd: ${facts.cwd}`);
  if (facts.activeFile) lines.push(`active_file: ${facts.activeFile}`);
  if (facts.activeTerminalPrivate) {
    lines.push("active_terminal_mode: private");
  }
  return formatEnvBlock(lines);
}

/** Isolated worktree run env (workspace path + branch). */
export function buildIsolatedWorktreeEnvBlock(
  workspacePath: string,
  branchName?: string | null,
): string {
  return `<env>\nworkspace_root: ${workspacePath}\nactive_branch: ${
    branchName ?? "(unknown)"
  }\nworkspace_mode: isolated-worktree\n</env>`;
}

/** Prepend env block to user text, or return text alone. */
export function prependEnvBlockToText(
  text: string,
  envBlock: string | null | undefined,
): string {
  return envBlock ? `${envBlock}\n\n${text}` : text;
}
