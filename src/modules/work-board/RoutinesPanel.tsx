import { useCallback, useEffect, useState } from "react";
import { formatRelativeTime } from "@altai/agent-ui";
import type { Routine } from "@altai/host-contract";
import { cn } from "@/lib/utils";
import { native } from "@/modules/ai/lib/native";
import {
  formatScheduleTime,
  projectRoutines,
  summarizeRoutines,
  type RoutineRow,
} from "./lib/routinesProjection";

type Props = {
  workspacePath: string;
  onOpenWork: (workId: string) => void;
  className?: string;
};

type LoadStatus = "loading" | "ready" | "error";

/**
 * Scheduled-work surface (package 066, PR 1). Routine aggregates are
 * durable in work.db — written by control-plane commands, materialized
 * by its cron bridge — and this surface reads them: what is scheduled,
 * when it fires next, and what it has already fired on. Read-only:
 * lifecycle moves stay control-plane commands. A routine whose next fire
 * is in the past is overdue — the bridge has not caught up on it — and
 * the surface says so instead of hiding it.
 */
export function RoutinesPanel({ workspacePath, onOpenWork, className }: Props) {
  const [status, setStatus] = useState<LoadStatus>("loading");
  const [error, setError] = useState<string | null>(null);
  const [routines, setRoutines] = useState<Routine[]>([]);

  const load = useCallback(async () => {
    try {
      const next = await native.routinesList(workspacePath);
      setRoutines(next);
      setError(null);
      setStatus("ready");
    } catch (loadError) {
      setError(
        loadError instanceof Error ? loadError.message : String(loadError),
      );
      setStatus("error");
    }
  }, [workspacePath]);

  useEffect(() => {
    setRoutines([]);
    setStatus("loading");
    void load();
  }, [load]);

  const rows = projectRoutines(routines, Date.now());
  const summary = summarizeRoutines(rows);

  return (
    <div
      className={cn(
        "flex h-full min-h-0 flex-col overflow-hidden bg-card",
        className,
      )}
    >
      <header className="flex shrink-0 items-baseline gap-2 border-b border-border-subtle px-3 py-2">
        <h2 className="min-w-0 flex-1 text-[13px] font-semibold text-foreground">
          Routines
        </h2>
        <p className="shrink-0 text-[10px] text-muted-foreground">
          Scheduled work, read from the control plane
        </p>
      </header>

      {status === "loading" ? (
        <p className="px-3 py-6 text-[11px] text-muted-foreground">
          Loading routines…
        </p>
      ) : null}
      {status === "error" ? (
        <div className="m-3 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-[11px] text-red-500">
          <p>{error ?? "Routines failed to load."}</p>
          <button
            type="button"
            onClick={() => void load()}
            className="mt-1 underline underline-offset-2"
          >
            Retry
          </button>
        </div>
      ) : null}

      {status === "ready" && rows.length === 0 ? (
        <p className="px-3 py-6 text-center text-[11px] text-muted-foreground">
          No routines yet — recurring and event-triggered work defined
          through the control plane lands here.
        </p>
      ) : null}

      {status === "ready" && rows.length > 0 ? (
        <>
          <p className="shrink-0 border-b border-border-subtle px-3 py-1.5 text-[10.5px] text-muted-foreground">
            {summary.totalCount} routines · {summary.activeCount} active
            {summary.pausedCount > 0
              ? ` · ${summary.pausedCount} paused`
              : ""}
            {summary.overdueCount > 0
              ? ` · ${summary.overdueCount} overdue`
              : ""}
          </p>
          <ul className="min-h-0 flex-1 divide-y divide-border-subtle overflow-y-auto">
            {rows.map((row) => (
              <RoutineRowItem
                key={row.id}
                row={row}
                onOpenWork={onOpenWork}
              />
            ))}
          </ul>
        </>
      ) : null}
    </div>
  );
}

function RoutineRowItem({
  row,
  onOpenWork,
}: {
  row: RoutineRow;
  onOpenWork: (workId: string) => void;
}) {
  const target = row.targetWorkTitle ?? row.targetWorkId;
  const body = (
    <>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[12px] text-foreground">
          {target ?? "No target Work"}
          {row.status !== "active" ? (
            <span className="ml-1.5 text-[10.5px] text-muted-foreground">
              · {row.statusLabel}
            </span>
          ) : null}
        </span>
        <span
          className={cn(
            "block truncate font-mono text-[10.5px]",
            row.isOverdue ? "text-red-500" : "text-muted-foreground",
          )}
        >
          {row.scheduleLabel ?? "no schedule recorded"}
          {row.nextFireMs != null
            ? ` · ${formatScheduleTime(row.nextFireMs, Date.now())}`
            : ""}
          {row.lastFiredMs != null
            ? ` · last ${formatRelativeTime(row.lastFiredMs)}`
            : ""}
        </span>
      </span>
      <span className="shrink-0 text-[10.5px] text-muted-foreground">
        {formatRelativeTime(row.updatedMs)}
      </span>
    </>
  );

  if (!row.drillable) {
    return (
      <li className="flex w-full items-baseline gap-2 px-3 py-1.5 text-left">
        {body}
      </li>
    );
  }
  return (
    <li>
      <button
        type="button"
        onClick={() => row.targetWorkId && onOpenWork(row.targetWorkId)}
        className="flex w-full items-baseline gap-2 px-3 py-1.5 text-left transition-colors hover:bg-muted/50"
        aria-label={`${row.scheduleLabel ?? "routine"} — ${row.targetWorkTitle}`}
      >
        {body}
      </button>
    </li>
  );
}
