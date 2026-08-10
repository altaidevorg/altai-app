/**
 * Pure Task Runs queue surface subtitle (A6.248).
 */

/** Queue-mode subtitle: live workers vs attention count. */
export function taskQueueSurfaceSubtitle(
  activeCount: number,
  attentionCount: number,
): string {
  return `${activeCount} working · ${attentionCount} need attention`;
}

/** Create-mode subtitle under Task Runs. */
export const TASK_CREATE_SURFACE_SUBTITLE =
  "Delegate an isolated run without leaving this conversation";
