/**
 * Pure task-run context availability flags (A6.246).
 */

/** Whether the editor active file is already attached as task context. */
export function isActiveFileInContext(
  activeFilePath: string | null | undefined,
  contextFiles: readonly string[],
): boolean {
  return Boolean(activeFilePath && contextFiles.includes(activeFilePath));
}

/** Whether non-private terminal text can be offered as task context. */
export function isTerminalContextAvailable(
  isPrivate: boolean,
  terminalText: string | null | undefined,
): boolean {
  return Boolean(!isPrivate && terminalText?.trim());
}

/** Whether a CWD or workspace root can supply workspace context. */
export function isWorkspaceContextAvailable(
  cwd: string | null | undefined,
  workspaceRoot: string | null | undefined,
): boolean {
  return Boolean(cwd ?? workspaceRoot);
}
