/**
 * Pure helpers for chat message href → path / web classification.
 * Wave 4 / A6.13 — host opens files / browser; this package stays DOM-free.
 */

/**
 * Resolve a possibly-relative path against a workspace or cwd root.
 *
 * Absolute paths (POSIX `/...` or Windows `C:\...`) are returned as-is.
 * Relative paths require a root; without one we throw rather than guess.
 */
export function resolveWorkspacePath(
  rawPath: string,
  cwd: string | null,
): string {
  if (rawPath.startsWith("/") || /^[a-zA-Z]:[\\/]/.test(rawPath)) {
    return rawPath;
  }
  if (!cwd) {
    throw new Error(
      `cannot resolve relative path "${rawPath}": no active workspace root. Pass an absolute path.`,
    );
  }
  const sep = cwd.includes("\\") && !cwd.includes("/") ? "\\" : "/";
  return cwd.endsWith(sep) ? `${cwd}${rawPath}` : `${cwd}${sep}${rawPath}`;
}

export function isWebHref(href: string): boolean {
  return /^(https?|mailto|tel):/i.test(href.trim());
}

/**
 * Convert a markdown href into an absolute filesystem path when it points at
 * a local file (absolute, `file://`, or workspace-relative). Returns null for
 * web URLs or unresolvable relatives.
 */
export function hrefToFilePath(
  href: string,
  workspaceRoot: string | null,
): string | null {
  const raw = href.trim();
  if (!raw || raw === "streamdown:incomplete-link") return null;
  if (isWebHref(raw)) return null;

  let path = raw;
  if (/^file:/i.test(path)) {
    try {
      path = decodeURIComponent(new URL(path).pathname);
      // Windows: file:///C:/Users/... → pathname `/C:/Users/...`
      if (/^\/[a-zA-Z]:[\\/]/.test(path)) path = path.slice(1);
    } catch {
      path = path.replace(/^file:\/\//i, "");
      // Same Windows drive-letter strip when `new URL` throws.
      if (/^\/[a-zA-Z]:[\\/]/.test(path)) path = path.slice(1);
    }
  }

  if (path.startsWith("/") || /^[a-zA-Z]:[\\/]/.test(path)) return path;

  if (!workspaceRoot) return null;
  try {
    return resolveWorkspacePath(path.replace(/^\.\//, ""), workspaceRoot);
  } catch {
    return null;
  }
}
