/**
 * Pure short labels for task-run permission mode chips (A6.254).
 */

export type TaskPermissionModeLike =
  | "ask"
  | "auto-edit"
  | "plan"
  | "bypass"
  | string;

/** Compact mode name shown on the Task Runs config row. */
export function taskPermissionModeShortLabel(
  mode: TaskPermissionModeLike,
): string {
  switch (mode) {
    case "ask":
      return "Ask";
    case "auto-edit":
      return "Auto-edit";
    case "plan":
      return "Plan";
    case "bypass":
      return "Bypass";
    default:
      return "Ask";
  }
}
