/**
 * Pure automation schedule chrome for Operations panel (A6.209).
 */

import { localDateTimeValue } from "../components/AutomationScheduleFields.js";

/** Default datetime-local value: five minutes ahead, seconds cleared. */
export function defaultAutomationAtValue(nowMs: number = Date.now()): string {
  const next = new Date(nowMs + 5 * 60_000);
  next.setSeconds(0, 0);
  return localDateTimeValue(next.getTime());
}

export type AutomationScheduleLike =
  | { kind: "at"; atMs: number }
  | { kind: "every"; everyMs: number }
  | { kind: string; [key: string]: unknown };

export type AutomationNextRunLike = {
  schedule: AutomationScheduleLike;
  lastRunAtMs?: number | null;
};

/**
 * Sort key for "next run": absolute at-time, periodic from last/now, else max.
 * `nowMs` is only used for interval schedules with no prior run.
 */
export function automationNextRunAtMs(
  item: AutomationNextRunLike,
  nowMs: number = Date.now(),
): number {
  const schedule = item.schedule;
  if (schedule.kind === "at" && typeof schedule.atMs === "number") {
    return schedule.atMs;
  }
  if (schedule.kind === "every" && typeof schedule.everyMs === "number") {
    return (item.lastRunAtMs ?? nowMs) + schedule.everyMs;
  }
  return Number.MAX_SAFE_INTEGER;
}
