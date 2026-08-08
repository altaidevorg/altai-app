/**
 * Pure helpers for the Chat project-target chip (A6.86).
 * Hosts supply workspace roots / trust; no vscode/Tauri.
 */

export type WorkspaceTargetInfo = {
  roots: readonly string[];
  trusted?: boolean;
  currentDir?: string;
};

export type ProjectTargetView = {
  name: string;
  path: string | null;
  kind: "local" | "github" | null;
  rootUri: string | null;
  /** True when more than one workspace root is open. */
  multiRoot: boolean;
};

export function canMountProjectTarget(flags: {
  workspaceInfo: boolean;
}): boolean {
  return flags.workspaceInfo;
}

/**
 * Map host workspace roots onto presentational ChatProjectTarget fields.
 * When `preferredRootUri` is one of `roots`, it becomes the selected root.
 */
export function projectTargetFromWorkspace(
  info: WorkspaceTargetInfo | null | undefined,
  preferredRootUri?: string | null,
): ProjectTargetView {
  if (!info || info.roots.length === 0) {
    return {
      name: "Choose a project",
      path: null,
      kind: null,
      rootUri: null,
      multiRoot: false,
    };
  }
  const multiRoot = info.roots.length > 1;
  const preferred =
    preferredRootUri && info.roots.includes(preferredRootUri)
      ? preferredRootUri
      : null;
  const rootUri = preferred ?? info.roots[0] ?? null;
  const pathFromUri = rootUri ? fsPathFromUri(rootUri) : null;
  // Only use host currentDir when it matches the selected root (or single root).
  const currentMatches =
    pathFromUri &&
    info.currentDir?.trim() &&
    (normalizePath(info.currentDir) === normalizePath(pathFromUri) || !multiRoot);
  const path =
    (currentMatches ? info.currentDir!.trim() : null) || pathFromUri;
  const name = basenamePath(path ?? rootUri ?? "Workspace");
  return {
    name,
    path,
    kind: "local",
    rootUri,
    multiRoot,
  };
}

export function basenamePath(path: string): string {
  const clean = path.replace(/[/\\]+$/, "");
  const parts = clean.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || clean || "Workspace";
}

/** Keep preferred root only while it remains in the open workspace set. */
export function retainPreferredRoot(
  preferred: string | null,
  roots: readonly string[],
): string | null {
  if (!preferred) {
    return null;
  }
  return roots.includes(preferred) ? preferred : null;
}

function normalizePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

function fsPathFromUri(uri: string): string | null {
  try {
    if (uri.startsWith("file://")) {
      const without = uri.slice("file://".length);
      return decodeURIComponent(without);
    }
  } catch {
    /* fall through */
  }
  return uri || null;
}
