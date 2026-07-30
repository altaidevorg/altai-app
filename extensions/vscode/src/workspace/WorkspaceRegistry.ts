import { realpath } from "node:fs/promises";
import { isAbsolute, relative, resolve, sep } from "node:path";

export type WorkspaceFolderRef = {
  readonly name: string;
  readonly fsPath: string;
  readonly scheme: string;
  readonly uri: string;
};

export type FolderPicker = {
  pick<T extends { label: string }>(items: readonly T[], options: { placeHolder: string }): Promise<T | undefined>;
};

export type CanonicalizePath = (path: string) => Promise<string>;

export class WorkspaceSelectionError extends Error {
  constructor(public readonly reason: "no_workspace" | "virtual_workspace" | "selection_cancelled" | "outside_workspace") {
    super(reason);
  }
}

/**
 * Owns the explicit workspace-folder choice required by multi-root workspaces.
 * It intentionally only deals in filesystem identities; no workspace settings
 * are read at this boundary.
 */
export class WorkspaceRegistry {
  constructor(
    private readonly folders: () => readonly WorkspaceFolderRef[],
    private readonly picker: FolderPicker,
    private readonly canonicalize: CanonicalizePath = canonicalizePath,
  ) {}

  async chooseFolder(preferred?: WorkspaceFolderRef): Promise<WorkspaceFolderRef> {
    const folders = this.folders();
    if (preferred) return this.assertRunnable(preferred);
    if (folders.length === 0) throw new WorkspaceSelectionError("no_workspace");
    if (folders.length === 1) return this.assertRunnable(folders[0]!);

    const selection = await this.picker.pick(
      folders.map((folder) => ({ label: folder.name, description: folder.fsPath, folder })),
      { placeHolder: "Choose the workspace folder for this ALTAI session" },
    );
    if (!selection) throw new WorkspaceSelectionError("selection_cancelled");
    return this.assertRunnable(selection.folder);
  }

  async hostIdentity(folder: WorkspaceFolderRef): Promise<string> {
    this.assertRunnable(folder);
    return this.canonicalize(folder.fsPath);
  }

  /** Resolves an editor/explorer resource to its one owning workspace root.
   * This avoids asking a multi-root picker after the user has already made an
   * explicit file/selection choice. */
  async folderForResource(resource: Pick<WorkspaceFolderRef, "fsPath" | "scheme">): Promise<WorkspaceFolderRef> {
    if (resource.scheme !== "file" && resource.scheme !== "vscode-remote") throw new WorkspaceSelectionError("virtual_workspace");
    let target: string;
    try {
      target = await this.canonicalize(resource.fsPath);
    } catch {
      throw new WorkspaceSelectionError("selection_cancelled");
    }
    for (const folder of this.folders()) {
      if (folder.scheme !== resource.scheme) continue;
      const root = await this.hostIdentity(folder);
      if (isInside(root, target)) return folder;
    }
    throw new WorkspaceSelectionError("outside_workspace");
  }

  private assertRunnable(folder: WorkspaceFolderRef): WorkspaceFolderRef {
    // Remote extension hosts use vscode-remote URIs but still execute against a
    // local filesystem path in that remote host. Other schemes are virtual.
    if (folder.scheme !== "file" && folder.scheme !== "vscode-remote") {
      throw new WorkspaceSelectionError("virtual_workspace");
    }
    return folder;
  }
}

function isInside(root: string, target: string): boolean {
  const rel = relative(resolve(root), resolve(target));
  // `relative` uses the host separator. On POSIX a backslash is a legal file
  // name character, while on Windows an absolute or parent path is rejected.
  return rel === "" || (rel !== ".." && !rel.startsWith(`..${sep}`) && !isAbsolute(rel));
}

export async function canonicalizePath(path: string): Promise<string> {
  try {
    return await realpath(path);
  } catch {
    // The extension host might be remote or use a filesystem provider that
    // cannot resolve symlinks. A normalized absolute path remains deterministic.
    return resolve(path);
  }
}
