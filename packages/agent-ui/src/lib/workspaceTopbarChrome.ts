/**
 * Pure helpers for capability-gating the shared WorkspaceTopbarActions
 * cluster (A6.95).
 */

export type WorkspaceTopbarFlags = {
  taskRuns: boolean;
  automations: boolean;
  inbox: boolean;
  /** When true, Work/Inbox cluster is joined by an available inspector control. */
  inspector?: boolean;
};

/**
 * Mount the Work / Inbox cluster only when at least one Operations domain
 * route is available, or a run inspector is available (no dead buttons).
 */
export function canMountWorkspaceTopbar(flags: WorkspaceTopbarFlags): boolean {
  return (
    flags.taskRuns ||
    flags.automations ||
    flags.inbox ||
    Boolean(flags.inspector)
  );
}

export function workspaceTopbarWorkOpen(
  surface: "chat" | "operations" | "settings",
  operationsView: string,
): boolean {
  return surface === "operations" && operationsView === "work";
}

export function workspaceTopbarInboxOpen(
  surface: "chat" | "operations" | "settings",
  operationsView: string,
): boolean {
  return surface === "operations" && operationsView === "inbox";
}
