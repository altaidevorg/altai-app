/**
 * Pure workspace path join (A6.162).
 * Host owns FS roots; package only normalizes the textual join.
 */

/**
 * Join `root` + `relative` into a single path string.
 * Strips trailing slashes/backslashes on the root (keeps a single separator).
 */
export function joinWorkspaceRelativePath(
  root: string,
  relative: string,
): string {
  return `${root.replace(/[\\/]+$/, "")}/${relative}`;
}
