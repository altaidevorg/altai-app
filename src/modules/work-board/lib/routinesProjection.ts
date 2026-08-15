import type { Routine } from "@altai/host-contract";

/**
 * Routines projection (package 066, PR 1). Routine aggregates are durable
 * in `work.db` — the control plane writes them, its cron bridge
 * materializes them into wakes — and this projection makes scheduled work
 * readable where Work lives. Ordering is by what happens next: overdue
 * recurring routines first, then upcoming ones, then event-triggered and
 * intent-less routines. Lifecycle stays a control-plane command; the
 * surface reads.
 */

export type RoutineRow = {
  id: string;
  status: Routine["status"];
  statusLabel: string;
  triggerKind: Routine["triggerKind"];
  /** The raw schedule fact: the cron expression or the event source. */
  scheduleLabel: string | null;
  targetWorkId: string | null;
  targetWorkTitle: string | null;
  /** A row is drillable only when its target Work resolved to a title. */
  drillable: boolean;
  lastFiredMs: number | null;
  nextFireMs: number | null;
  isOverdue: boolean;
  updatedMs: number;
};

export type RoutinesSummary = {
  totalCount: number;
  activeCount: number;
  pausedCount: number;
  overdueCount: number;
};

/** Format a schedule boundary relative to `nowMs`: past is overdue,
 *  future is upcoming. Minutes are the finest unit a cron routine can
 *  express, so nothing finer is shown. */
export function formatScheduleTime(timestamp: number, nowMs: number): string {
  const deltaMs = timestamp - nowMs;
  const absMinutes = Math.floor(Math.abs(deltaMs) / 60_000);
  if (absMinutes < 1) return "just now";
  const magnitude =
    absMinutes < 60
      ? `${absMinutes}m`
      : absMinutes < 1440
        ? `${Math.floor(absMinutes / 60)}h`
        : `${Math.floor(absMinutes / 1440)}d`;
  return deltaMs < 0 ? `${magnitude} overdue` : `in ${magnitude}`;
}

export function toRoutineRow(routine: Routine, nowMs: number): RoutineRow {
  const scheduleLabel =
    routine.triggerKind === "recurring"
      ? routine.cronExpression ?? null
      : routine.eventSource ?? null;
  const nextFireMs = routine.nextFireAtMs ?? null;
  return {
    id: routine.id,
    status: routine.status,
    statusLabel: routine.status,
    triggerKind: routine.triggerKind,
    scheduleLabel,
    targetWorkId: routine.targetWorkId ?? null,
    targetWorkTitle: routine.targetWorkTitle ?? null,
    drillable: routine.targetWorkTitle != null,
    lastFiredMs: routine.lastFiredAtMs ?? null,
    nextFireMs,
    isOverdue: nextFireMs != null && nextFireMs <= nowMs,
    updatedMs: routine.updatedAtMs,
  };
}

/** Project the workspace's routines into display order: recurring
 *  routines by their next fire (overdue naturally sorts first), then
 *  event-triggered routines, then routines with no scheduled intent. */
export function projectRoutines(
  routines: readonly Routine[],
  nowMs: number,
): RoutineRow[] {
  const rows = routines.map((routine) => toRoutineRow(routine, nowMs));
  return rows.sort((a, b) => {
    const rank = (row: RoutineRow): number => {
      if (row.nextFireMs != null) return 0;
      return row.scheduleLabel != null ? 1 : 2;
    };
    const byKind = rank(a) - rank(b);
    if (byKind !== 0) return byKind;
    if (a.nextFireMs != null && b.nextFireMs != null) {
      return a.nextFireMs - b.nextFireMs;
    }
    return b.updatedMs - a.updatedMs;
  });
}

export function summarizeRoutines(rows: readonly RoutineRow[]): RoutinesSummary {
  return {
    totalCount: rows.length,
    activeCount: rows.filter((row) => row.status === "active").length,
    pausedCount: rows.filter((row) => row.status === "paused").length,
    overdueCount: rows.filter((row) => row.isOverdue).length,
  };
}
