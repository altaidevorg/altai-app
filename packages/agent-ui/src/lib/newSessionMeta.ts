/**
 * Pure untitled session meta factory (A6.187).
 */

import { DEFAULT_SESSION_TITLE } from "./backendSessionTitle.js";

export type UntitledSessionMetaSeed = {
  id: string;
  now?: number;
  workspacePath?: string | null;
  workspaceKind?: "local" | "github" | null;
  repositoryUrl?: string | null;
};

export type UntitledSessionMeta = {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  workspacePath?: string | null;
  workspaceKind?: "local" | "github" | null;
  repositoryUrl?: string | null;
};

/** Build a default “New chat” session meta row. */
export function newUntitledSessionMeta(
  seed: UntitledSessionMetaSeed,
): UntitledSessionMeta {
  const now = seed.now ?? Date.now();
  return {
    id: seed.id,
    title: DEFAULT_SESSION_TITLE,
    createdAt: now,
    updatedAt: now,
    ...(seed.workspacePath !== undefined
      ? { workspacePath: seed.workspacePath }
      : {}),
    ...(seed.workspaceKind !== undefined
      ? { workspaceKind: seed.workspaceKind }
      : {}),
    ...(seed.repositoryUrl !== undefined
      ? { repositoryUrl: seed.repositoryUrl }
      : {}),
  };
}
