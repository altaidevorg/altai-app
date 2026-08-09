/**
 * Pure session workspace target patch (A6.194).
 */

export type SessionWorkspaceTarget = {
  path: string | null;
  kind: "local" | "github" | null;
  repositoryUrl?: string | null;
};

export type SessionWorkspaceFields = {
  id: string;
  workspacePath?: string | null;
  workspaceKind?: "local" | "github" | null;
  repositoryUrl?: string | null;
  updatedAt: number;
};

/** Apply workspace target fields onto a matching session row. */
export function applySessionWorkspaceTarget<T extends SessionWorkspaceFields>(
  sessions: readonly T[],
  id: string,
  target: SessionWorkspaceTarget,
  now: number = Date.now(),
): T[] {
  return sessions.map((session) =>
    session.id === id
      ? {
          ...session,
          workspacePath: target.path,
          workspaceKind: target.kind,
          repositoryUrl: target.repositoryUrl ?? null,
          updatedAt: now,
        }
      : session,
  );
}
