/**
 * Pure Desktop composer placeholder / follow-up chrome (A6.255).
 */

/** Idle vs busy main textarea placeholder. */
export function composerDesktopPlaceholder(isBusy: boolean): string {
  return isBusy
    ? "Add a follow-up, steer the active run, or queue the next task…"
    : "Describe a task or ask a follow-up…  @ files  / commands  # snippets";
}

/** Hint strip under the composer while a run is active. */
export function composerFollowupBarHint(input: {
  isCancelling: boolean;
  canSteer: boolean;
}): string {
  if (input.isCancelling) {
    return "Cancellation requested — you can queue the next task";
  }
  return input.canSteer
    ? "Enter queues next · ⌘/Ctrl+Enter steers this run"
    : "Enter queues next · starts after the active run ends";
}

/** Steer control title (blocked when media attachments present). */
export function composerSteerControlTitle(blockedByMedia: boolean): string {
  return blockedByMedia
    ? "Steering cannot include images or PDFs; use Queue next"
    : "Apply at the active run's next safe boundary";
}

export const COMPOSER_QUEUE_CONTROL_TITLE =
  "Start after the active run terminates";
