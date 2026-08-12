import { ask, open } from "@tauri-apps/plugin-dialog";
import { createAppStore } from "@/lib/appStore";
import { create } from "zustand";
import { native } from "../ai/lib/native";

/**
 * Transient mirror of the active conversation's optional project target.
 * Recents persist, but the active folder does not: a chat starts project-free
 * unless that conversation's own metadata attaches a local or cloned repo.
 */
const STORE_PATH = "altai-workspace.json";
const KEY_FOLDER = "folder";
const KEY_RECENTS = "recents";
const HYDRATION_TIMEOUT_MS = 3_000;
// How many recent workspaces the welcome screen remembers. Cursor shows a
// similar short list — enough to jump back to active projects, not a history.
const RECENTS_CAP = 12;

const store = createAppStore(STORE_PATH, { defaults: {}, autoSave: 200 });

function withHydrationDeadline<T>(operation: Promise<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => {
      reject(new Error("workspace store hydration timed out"));
    }, HYDRATION_TIMEOUT_MS);

    void operation
      .then(resolve, reject)
      .finally(() => window.clearTimeout(timer));
  });
}

function prependRecent(recents: string[], path: string): string[] {
  return [path, ...recents.filter((p) => p !== path)].slice(0, RECENTS_CAP);
}

/**
 * Mirror the recents into the native OS menu (macOS Dock / Windows Jump List)
 * so they're reachable by right-clicking the app icon. Best-effort: the command
 * is a no-op on Linux and absent outside Tauri.
 */
function pushRecentFolders(folders: string[]): void {
  void native.setRecentFolders(folders).catch(() => {});
}

/**
 * Whether `path` still exists and is a directory. Used to fall back to the
 * welcome screen — instead of loading into a broken workspace ("no such file or
 * directory") — when a persisted or recent folder was deleted/moved/unmounted.
 */
async function folderIsAccessible(path: string): Promise<boolean> {
  try {
    // Authorize first: fs access for paths outside the default scope must go
    // through workspace authorization, else stat fails for a perfectly valid
    // folder. A missing path makes authorize/stat throw → treated as gone.
    await native.workspaceAuthorize(path);
    return (await native.stat(path)).kind === "dir";
  } catch {
    return false;
  }
}

type State = {
  folder: string | null;
  /** Most-recently-opened workspaces, newest first. Powers the welcome list. */
  recents: string[];
  hydrated: boolean;
  /**
   * Set when the active workspace was just produced by a clone, so the app can
   * open straight into the Source Control view instead of the file explorer.
   * Transient (never persisted); consumed + cleared by the app on mount.
   */
  justCloned: boolean;
  clearJustCloned: () => void;
  hydrate: () => Promise<void>;
  setFolder: (path: string) => void;
  /** Open one user-selected folder and replace the exact preview grant. */
  openFolder: (path: string) => Promise<string>;
  /** Switch from metadata/restored context without creating an exact grant. */
  switchFolder: (path: string) => Promise<string>;
  /** Open the native directory picker; adds + returns the chosen path. */
  pickFolder: () => Promise<string | null>;
  /**
   * Prompt for a destination directory, clone `url` into it via the Rust
   * `git_clone` command, then open the cloned repo as the workspace. Returns
   * the cloned path, or null if the user cancelled the destination dialog.
   * Throws (with git's error text) if the clone itself fails.
   */
  cloneRepo: (url: string) => Promise<string | null>;
  /** Drop a path from the recents list (e.g. it was moved/deleted). */
  removeRecent: (path: string) => void;
  /**
   * Open a workspace from the recents list. Verifies it still exists first; if
   * it was deleted/moved, drops it from recents instead of loading into an
   * error screen. Resolves true when the folder was opened.
   */
  openRecent: (path: string) => Promise<boolean>;
  /**
   * Clear the transient folder mirror while keeping the recent-project list.
   */
  closeFolder: () => Promise<void>;
};

export const useWorkspaceFolderStore = create<State>((set, get) => ({
  folder: null,
  recents: [],
  hydrated: false,
  justCloned: false,
  clearJustCloned: () => {
    if (get().justCloned) set({ justCloned: false });
  },
  hydrate: async () => {
    if (get().hydrated) return;
    let recentList: string[] = [];
    try {
      recentList = await withHydrationDeadline(
        (async () => {
          const recents = (await store.get<string[]>(KEY_RECENTS)) ?? [];
          const normalized = Array.isArray(recents) ? recents : [];
          // Agent Workspace starts project-free. Workspace targets are restored
          // from each conversation's metadata, never inherited globally from the
          // last IDE session.
          await store.delete(KEY_FOLDER);
          await store.save();
          return normalized;
        })(),
      );
    } catch (error) {
      // A missing/corrupt store or a transient native-plugin failure must never
      // leave the entire app permanently blank. Start clean and let the user
      // attach a project later if needed.
      console.warn("workspace hydration failed; starting without a workspace", error);
    }
    set({
      folder: null,
      recents: recentList,
      hydrated: true,
    });
    pushRecentFolders(recentList);
  },
  setFolder: (path) => {
    const recents = prependRecent(get().recents, path);
    set({ folder: path, recents });
    pushRecentFolders(recents);
    // Conversation metadata owns target persistence. The folder store only
    // mirrors the active chat for Explorer/terminal integration.
    void (async () => {
      await store.set(KEY_RECENTS, recents);
      await store.save();
    })();
  },
  openFolder: async (path) => {
    const canonical = await native.workspaceAuthorizeOpened(path);
    get().setFolder(canonical);
    return canonical;
  },
  switchFolder: async (path) => {
    // Automatic metadata/URL/environment switches are not user-open proof.
    // Revoke the prior exact grant, then retain only broad filesystem access.
    await native.workspaceRevokeOpened();
    const canonical = await native.workspaceAuthorize(path);
    get().setFolder(canonical);
    return canonical;
  },
  pickFolder: async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select workspace folder",
    });
    if (typeof selected === "string") {
      return get().openFolder(selected);
    }
    return null;
  },
  cloneRepo: async (url) => {
    const trimmed = url.trim();
    if (!trimmed) throw new Error("Enter a repository URL.");
    const parent = await open({
      directory: true,
      multiple: false,
      title: "Choose where to clone",
    });
    if (typeof parent !== "string") return null; // cancelled
    const dest = await native.gitClone(trimmed, parent);
    const canonical = await get().openFolder(dest);
    set({ justCloned: true });
    return canonical;
  },
  removeRecent: (path) => {
    const recents = get().recents.filter((p) => p !== path);
    set({ recents });
    pushRecentFolders(recents);
    void (async () => {
      await store.set(KEY_RECENTS, recents);
      await store.save();
    })();
  },
  openRecent: async (path) => {
    if (await folderIsAccessible(path)) {
      // The probe above is intentionally broad-only. Clicking a valid recent
      // entry is the actual open action that replaces the exact grant.
      await get().openFolder(path);
      return true;
    }
    // Folder is gone — confirm before pruning so a temporarily-unplugged drive
    // or offline network share doesn't silently lose the entry.
    const remove = await ask(
      `"${path}" is no longer accessible.\n\nRemove it from recent projects?`,
      { title: "Folder not found", kind: "warning" },
    );
    if (remove) get().removeRecent(path);
    return false;
  },
  closeFolder: async () => {
    await native.workspaceRevokeOpened();
    set({ folder: null });
    await store.delete(KEY_FOLDER);
    await store.save();
  },
}));

export function currentWorkspaceFolder(): string | null {
  return useWorkspaceFolderStore.getState().folder;
}

/** Last path segment of a workspace folder, for display. */
export function folderName(path: string): string {
  const normalized = path.replace(/[/\\]+$/, "");
  const idx = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  return idx >= 0 ? normalized.slice(idx + 1) : normalized;
}

/** Collapse the home prefix to `~` so recent paths read like a shell would. */
export function prettyDir(path: string, home: string | null): string {
  if (!home) return path;
  return path === home
    ? "~"
    : path.startsWith(`${home}/`)
      ? `~${path.slice(home.length)}`
      : path;
}
