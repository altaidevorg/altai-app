/**
 * Pure classifier for whether the host can treat open folders as a real FS root (A6.118).
 */

const LOCAL_LIKE_SCHEMES = new Set([
  "file",
  "vscode-remote",
  "vscode-userdata",
]);

/**
 * True when every open folder lacks a local/remote machine path the native
 * host can execute against (e.g. pure virtual vscode-vfs / vscode.dev).
 */
export function isVirtualOnlyWorkspace(
  folders: readonly { scheme: string; fsPath?: string }[],
): boolean {
  if (folders.length === 0) {
    return false;
  }
  return folders.every((folder) => {
    const scheme = folder.scheme.trim().toLowerCase();
    if (!scheme || scheme === "untitled") {
      return true;
    }
    if (LOCAL_LIKE_SCHEMES.has(scheme)) {
      return false;
    }
    // Non-empty fsPath with file-looking path still treat as real if scheme ok;
    // other schemes with empty path are virtual-only.
    const path = folder.fsPath?.trim() ?? "";
    return path.length === 0 || scheme.startsWith("vscode-");
  });
}
