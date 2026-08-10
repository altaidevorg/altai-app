/**
 * Pure Automation create form schedule validation copy (A6.211).
 */

import type { AutomationScheduleMode } from "../components/AutomationScheduleFields.js";

export type { AutomationScheduleMode };

/**
 * Human-readable schedule field error, or null when the draft is sendable.
 * `at` requires a finite future timestamp; `every` requires minutes >= 1.
 */
export function automationScheduleError(
  mode: AutomationScheduleMode,
  atMs: number,
  everyMinutes: number,
  nowMs: number = Date.now(),
): string | null {
  if (mode === "at") {
    if (!Number.isFinite(atMs) || atMs <= nowMs) {
      return "Choose a valid future time";
    }
    return null;
  }
  if (!Number.isFinite(everyMinutes) || everyMinutes < 1) {
    return "Minimum interval is 1 minute";
  }
  return null;
}
