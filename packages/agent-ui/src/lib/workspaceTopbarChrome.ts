/**
 * Pure helpers for capability-gating the shared run-details control (A6.95).
 * Work / Inbox destinations live in primary Desktop / Operations navigation.
 */

export type WorkspaceTopbarFlags = {
  taskRuns: boolean;
  automations: boolean;
  inbox: boolean;
  /** When true, the active chat may open the run inspector. */
  inspector?: boolean;
};

/**
 * Mount run-details chrome when an inspector is available for the session.
 * Legacy Operations flags remain for host capability matrices.
 */
export function canMountWorkspaceTopbar(flags: WorkspaceTopbarFlags): boolean {
  return Boolean(flags.inspector);
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
