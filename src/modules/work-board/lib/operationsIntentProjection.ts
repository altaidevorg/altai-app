/**
 * Operations-intent mapping (package 067, PR 1). Chat shortcuts dispatch
 * `altai:open-operations` intents (`view`, optional `workHubView`) — the
 * same vocabulary in studio and desktop hosts. This mapping lands those
 * intents on the Work OS Home surface they name, so a chat shortcut and
 * a click on Home's tabs are one navigation vocabulary, and the chat
 * owns no durable state of its own: it points at canonical surfaces.
 */

export type OperationsIntentView = "overview" | "work" | "runs" | "inbox";
export type OperationsWorkHubView = "runs" | "scheduled";
export type HomeSurface = "work" | "agents" | "audit" | "routines";

/** The Home surface an operations intent opens. Scheduled work is the
 *  Routines surface; every other view (overview, work, runs, inbox)
 *  lands on the Work surface, whose left column already carries the
 *  inbox and the runs hub. The view itself names a section of that
 *  column rather than a separate surface, so only the hub split below
 *  decides between the two. */
export function homeSurfaceFromOperationsIntent(
  _view: OperationsIntentView,
  workHubView?: OperationsWorkHubView | null,
): HomeSurface {
  if (workHubView === "scheduled") return "routines";
  return "work";
}
