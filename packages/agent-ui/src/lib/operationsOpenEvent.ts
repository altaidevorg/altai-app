/**
 * Pure Operations navigation intents shared by Desktop chrome and VS Code (A6.136).
 * Hosts dispatch CustomEvent / deep-link payloads; this never touches window.
 */

export type OperationsOpenView = "overview" | "work" | "runs" | "inbox";
export type OperationsOpenWorkHubView = "runs" | "scheduled";

export type OperationsOpenIntent = {
  view: OperationsOpenView;
  workHubView?: OperationsOpenWorkHubView;
};

/**
 * Build a stable intent for opening Work / Inbox from AI side-panel chrome.
 * Canonical Work/Inbox live under Operations (not AI overlays).
 */
export function buildOperationsOpenIntent(
  view: OperationsOpenView,
  workHubView?: OperationsOpenWorkHubView,
): OperationsOpenIntent {
  if (view === "work") {
    return {
      view: "work",
      workHubView: workHubView ?? "runs",
    };
  }
  return { view };
}
