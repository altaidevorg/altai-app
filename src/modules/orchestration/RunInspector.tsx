/**
 * Run Inspector component (C2) — event timeline, session analysis,
 * and support bundle export.
 */

import { useEffect } from "react";
import { useInspectorStore } from "./inspectorStore";
import type { AttemptOutcome } from "@/modules/ai/lib/native";
import { cn } from "@/lib/utils";

const OUTCOME_COLORS: Record<AttemptOutcome, string> = {
  success: "text-green-600",
  failure: "text-red-600",
  expensive: "text-orange-600",
  abandoned: "text-gray-500",
};

function formatMs(ms: number | null): string {
  if (ms == null) return "—";
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60_000).toFixed(1)}m`;
}

export function RunInspector({
  dbPath,
  workspaceKey,
}: {
  dbPath: string;
  workspaceKey: string;
}) {
  const {
    analyses,
    bundle,
    loading,
    error,
    selectedTaskId,
    loadAnalysis,
    exportBundle,
    selectTask,
  } = useInspectorStore();

  useEffect(() => {
    void loadAnalysis(dbPath, workspaceKey);
  }, [dbPath, workspaceKey, loadAnalysis]);

  if (loading && analyses.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        Loading analysis…
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

  const selected = analyses.find((a) => a.taskId === selectedTaskId);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b px-4 py-2">
        <span className="text-sm font-medium">Run Inspector</span>
        <button
          className="rounded-md border bg-background px-2 py-1 text-xs hover:bg-accent"
          onClick={() =>
            void exportBundle(
              dbPath,
              workspaceKey,
              selectedTaskId ? [selectedTaskId] : [],
            )
          }
        >
          Export Bundle
        </button>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* Task list */}
        <div className="w-64 shrink-0 overflow-y-auto border-r">
          {analyses.map((a) => (
            <button
              key={a.taskId}
              className={cn(
                "flex w-full items-center justify-between px-3 py-2 text-left text-xs hover:bg-accent",
                selectedTaskId === a.taskId && "bg-accent",
              )}
              onClick={() => selectTask(a.taskId)}
            >
              <span className="truncate">{a.taskId}</span>
              <span className={cn("ml-2 shrink-0", OUTCOME_COLORS[a.outcome])}>
                {a.outcome}
              </span>
            </button>
          ))}
          {analyses.length === 0 && (
            <div className="px-3 py-4 text-center text-xs text-muted-foreground">
              No tasks analyzed
            </div>
          )}
        </div>

        {/* Detail panel */}
        <div className="flex-1 overflow-y-auto p-4">
          {selected ? (
            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-medium">{selected.taskId}</h3>
                <p className={cn("text-xs", OUTCOME_COLORS[selected.outcome])}>
                  Outcome: {selected.outcome}
                </p>
              </div>

              <div className="grid grid-cols-3 gap-2 text-xs">
                <div className="rounded-md border p-2">
                  <div className="text-muted-foreground">Attempts</div>
                  <div className="font-medium">{selected.attemptCount}</div>
                </div>
                <div className="rounded-md border p-2">
                  <div className="text-muted-foreground">Duration</div>
                  <div className="font-medium">
                    {formatMs(selected.durationMs)}
                  </div>
                </div>
                <div className="rounded-md border p-2">
                  <div className="text-muted-foreground">Error</div>
                  <div className="truncate font-medium">
                    {selected.errorSummary ?? "—"}
                  </div>
                </div>
              </div>

              {selected.signals.length > 0 && (
                <div>
                  <h4 className="mb-1 text-xs font-medium">Signals</h4>
                  <div className="space-y-1">
                    {selected.signals.map((s, i) => (
                      <div
                        key={i}
                        className="flex items-center gap-2 rounded-md bg-muted/50 px-2 py-1 text-xs"
                      >
                        <span className="rounded bg-muted px-1.5 py-0.5 text-[10px]">
                          {s.kind}
                        </span>
                        <span className="text-muted-foreground">{s.detail}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              Select a task to inspect
            </div>
          )}
        </div>
      </div>

      {/* Bundle preview */}
      {bundle && (
        <div className="max-h-40 overflow-y-auto border-t p-3 text-xs">
          <div className="mb-1 flex items-center gap-2">
            <span className="font-medium">Support Bundle</span>
            <span className="rounded bg-muted px-1.5 py-0.5 text-[10px]">
              {bundle.sanitized ? "sanitized" : "raw"}
            </span>
            <span className="text-muted-foreground">
              {bundle.events.length} events
            </span>
          </div>
          <p className="text-[10px] text-muted-foreground">
            Exports from the UI are always sanitized.
          </p>
        </div>
      )}
    </div>
  );
}
