import { invoke } from "@tauri-apps/api/core";

export type LaunchPayload = {
  type: "file" | "folder" | "multi_file";
  paths: string[];
  action?: "explain" | "refactor" | "ask-project";
};

let pending: LaunchPayload[] = [];

const LAUNCH_READ_TIMEOUT_MS = 3_000;

function readPendingLaunches(): Promise<LaunchPayload[]> {
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      console.warn("initial launch payload read timed out; continuing startup");
      resolve([]);
    }, LAUNCH_READ_TIMEOUT_MS);

    void invoke<LaunchPayload[]>("get_pending_launches")
      .then(resolve, () => resolve([]))
      .finally(() => window.clearTimeout(timer));
  });
}

export async function initPendingLaunches(): Promise<void> {
  const launches = await readPendingLaunches();
  pending = launches.map((l) => ({
    ...l,
    paths: l.paths.map((p) => p.replace(/\\/g, "/")),
  }));
}

export function getInitialLaunches(): LaunchPayload[] {
  const result = [...pending];
  pending = [];
  return result;
}

/**
 * Returns the best-guess initial directory for the first terminal.
 * Prefers an explicitly opened folder; falls back to the parent directory
 * of the first explicitly opened file.
 */
export function getLaunchDir(): string | undefined {
  const folder = pending.find((l) => l.type === "folder");
  if (folder) return folder.paths[0];

  const file = pending.find((l) => l.type === "file" || l.type === "multi_file");
  if (file && file.paths[0]) {
    const p = file.paths[0];
    const idx = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
    return idx >= 0 ? p.slice(0, idx) : undefined;
  }
  return undefined;
}

/** Folder handed to a freshly opened IDE window via `?folder=`. */
export function getStudioFolderFromUrl(
  search = typeof window !== "undefined" ? window.location.search : "",
): string | null {
  try {
    const raw = new URLSearchParams(search).get("folder");
    if (!raw) return null;
    const decoded = decodeURIComponent(raw).trim().replace(/\\/g, "/");
    return decoded.length > 0 ? decoded : null;
  } catch {
    return null;
  }
}
