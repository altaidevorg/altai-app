import { invoke } from "@tauri-apps/api/core";

export type LaunchPayload = {
  type: "file" | "folder" | "multi_file";
  paths: string[];
  action?: "explain" | "refactor" | "ask-project";
};

let pending: LaunchPayload[] = [];

export async function initPendingLaunches(): Promise<void> {
  const launches = await invoke<LaunchPayload[]>("get_pending_launches").catch(
    () => [],
  );
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
