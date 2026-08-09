/**
 * Pure multi-root session query targets (A6.182).
 * Project-free store (undefined) plus unique workspace paths from frontend list.
 */

export type SessionWorkspacePathLike = {
  workspacePath?: string | null;
};

/** List native session-store targets to query for recovery merge. */
export function sessionListWorkspaceTargets(
  frontend: readonly SessionWorkspacePathLike[],
): Array<string | undefined> {
  return [
    undefined,
    ...new Set(
      frontend
        .map((session) => session.workspacePath ?? undefined)
        .filter((path): path is string => Boolean(path)),
    ),
  ];
}

/** Resolve a session id to its optional workspace path. */
export function sessionWorkspacePathForId(
  sessions: readonly (SessionWorkspacePathLike & { id: string })[],
  id: string,
): string | undefined {
  return sessions.find((session) => session.id === id)?.workspacePath ?? undefined;
}
