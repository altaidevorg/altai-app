/**
 * Pure Task Context source detail copy (A6.234).
 */

/** Detail line under the terminal context toggle. */
export function terminalContextDetailLabel(input: {
  terminalPrivate: boolean;
  terminalAvailable: boolean;
}): string {
  if (input.terminalPrivate) {
    return "Unavailable while the active terminal is private";
  }
  if (input.terminalAvailable) {
    return "Latest visible output from the active terminal";
  }
  return "No terminal output available";
}

/** Detail line under the Git diff context toggle. */
export function gitDiffContextDetailLabel(workspaceAvailable: boolean): string {
  return workspaceAvailable
    ? "Current unstaged Git diff"
    : "Open a workspace to include Git changes";
}
