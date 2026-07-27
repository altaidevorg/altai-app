/**
 * Task Board component (C1) — Kanban-style board showing orchestration tasks
 * grouped by status column with quality metrics summary.
 */

import { useEffect } from "react";
import { useBoardStore, type BoardColumn, type BoardTask } from "./boardStore";
import { cn } from "@/lib/utils";

const COLUMN_LABELS: Record<BoardColumn, string> = {
  queued: "Queued",
  running: "Running",
  reviewing: "Reviewing",
  done: "Done",
  blocked: "Blocked",
};

const COLUMN_ORDER: BoardColumn[] = [
  "queued",
  "running",
  "reviewing",
  "done",
  "blocked",
];

const PRIORITY_COLORS: Record<BoardTask["priority"], string> = {
  critical: "border-l-red-500",
  high: "border-l-orange-500",
  normal: "border-l-blue-500",
  low: "border-l-gray-400",
};

function formatDuration(ms: number | null): string {
  if (ms == null) return "—";
  if (ms < 60_000) return `${Math.round(ms / 1000)}s`;
  if (ms < 3_600_000) return `${Math.round(ms / 60_000)}m`;
  return `${(ms / 3_600_000).toFixed(1)}h`;
}

function formatRate(rate: number | null): string {
  if (rate == null) return "—";
  return `${(rate * 100).toFixed(0)}%`;
}

export function TaskBoard({
  workspaceKey,
  dbPath,
}: {
  workspaceKey: string;
  dbPath: string;
}) {
  const { tasks, metrics, loading, error, load } = useBoardStore();

  useEffect(() => {
    load(workspaceKey, dbPath);
  }, [workspaceKey, dbPath, load]);

  if (loading && tasks.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        Loading board…
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-destructive">
        {error}
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      {metrics && (
        <div className="flex items-center gap-4 border-b px-4 py-2 text-xs text-muted-foreground">
          <span>Success: {formatRate(metrics.firstAttemptSuccessRate)}</span>
          <span>Retry: {formatRate(metrics.retryRate)}</span>
          <span>Median handoff: {formatDuration(metrics.medianTimeToHandoffMs)}</span>
        </div>
      )}
      <div className="flex flex-1 gap-2 overflow-x-auto p-2">
        {COLUMN_ORDER.map((column) => {
          const columnTasks = tasks.filter((t) => t.column === column);
          return (
            <div
              key={column}
              className="flex w-64 shrink-0 flex-col rounded-lg border bg-muted/30"
            >
              <div className="flex items-center justify-between px-3 py-2 text-xs font-medium">
                {COLUMN_LABELS[column]}
                <span className="rounded-full bg-muted px-1.5 py-0.5 text-[10px]">
                  {columnTasks.length}
                </span>
              </div>
              <div className="flex flex-1 flex-col gap-1.5 overflow-y-auto p-2">
                {columnTasks.map((task) => (
                  <div
                    key={task.taskId}
                    className={cn(
                      "rounded-md border border-l-4 bg-background p-2 text-xs shadow-sm",
                      PRIORITY_COLORS[task.priority],
                    )}
                  >
                    <div className="font-medium">{task.title}</div>
                    {task.attemptCount > 1 && (
                      <div className="mt-0.5 text-[10px] text-orange-600">
                        {task.attemptCount} attempts
                      </div>
                    )}
                    {task.blockedReason && task.blockedReason.length > 0 && (
                      <div className="mt-0.5 text-[10px] text-red-600">
                        Blocked by: {task.blockedReason.join(", ")}
                      </div>
                    )}
                  </div>
                ))}
                {columnTasks.length === 0 && (
                  <div className="py-4 text-center text-[10px] text-muted-foreground">
                    Empty
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
