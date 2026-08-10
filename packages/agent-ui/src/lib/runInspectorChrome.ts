/**
 * Pure Run details inspector copy for Desktop overview chrome (A6.250).
 */

/** Header subtitle while idle / working / stepped. */
export function runInspectorHeaderSubtitle(
  status: string,
  step?: string | null,
): string {
  if (status === "idle") {
    return "Ready for the next task";
  }
  return step ?? "Agent is working";
}

/** Absolute token total label for RunOverviewCard. */
export function runInspectorUsageTokenLabel(tokenTotal: number): string {
  return tokenTotal > 0
    ? `${tokenTotal.toLocaleString()} tokens`
    : "No usage yet";
}

/** Plan metric fraction or em dash when empty. */
export function planProgressMetricValue(
  completed: number,
  total: number,
): string {
  return total > 0 ? `${completed}/${total}` : "—";
}

/** Plan section summary under inspector collapse. */
export function planInspectorSectionSummary(
  completed: number,
  total: number,
): string {
  return total > 0
    ? `${completed} of ${total} steps complete`
    : "No checklist for this run";
}
