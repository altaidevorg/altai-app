import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";
import { RunStateMetric } from "./RunStateMetric.js";

export type ActivityInspectorEvent = {
  id: string;
  label: string;
  detail?: string;
  tone?: "default" | "success" | "warning" | "error";
  createdAt: number;
};

export type ActivityInspectorProps = {
  events: ActivityInspectorEvent[];
  hasQuery: boolean;
  compact?: boolean;
  /** Host-rendered status pill for the non-compact header. */
  statusPill?: ReactNode;
  step?: string | null;
  error?: string | null;
  approvalsPending?: number;
  subagentCount?: number;
  inputTokens?: number;
  outputTokens?: number;
};

/**
 * Run activity / timeline inspector. Purely presentational; the host maps
 * store meta into props and supplies the status pill when needed.
 */
export function ActivityInspector({
  events,
  hasQuery,
  compact = false,
  statusPill,
  step = null,
  error = null,
  approvalsPending = 0,
  subagentCount = 0,
  inputTokens = 0,
  outputTokens = 0,
}: ActivityInspectorProps) {
  const tokenTotal = inputTokens + outputTokens;

  return (
    <div className="space-y-2">
      {!compact ? (
        <>
          <section className="rounded-md border border-border bg-muted/40 p-2.5">
            <div className="flex items-center gap-2">
              {statusPill}
              <span className="ml-auto text-[10px] tabular-nums text-muted-foreground">
                {tokenTotal > 0
                  ? `${tokenTotal.toLocaleString()} tokens`
                  : "No tokens yet"}
              </span>
            </div>
            {step ? (
              <p className="mt-2 text-[11px] leading-relaxed text-muted-foreground">
                {step}
              </p>
            ) : null}
          </section>
          <section className="rounded-md border border-border bg-muted/30 p-2.5">
            <div className="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
              Run state
            </div>
            <div className="mt-2 grid grid-cols-2 gap-2 text-[11px]">
              <RunStateMetric
                label="Approvals"
                value={String(approvalsPending)}
              />
              <RunStateMetric
                label="Subagents"
                value={String(subagentCount)}
              />
              <RunStateMetric
                label="Input"
                value={inputTokens.toLocaleString()}
              />
              <RunStateMetric
                label="Output"
                value={outputTokens.toLocaleString()}
              />
            </div>
          </section>
          {error ? (
            <section className="border border-destructive/30 bg-destructive/[0.06] p-2.5 text-[11px] text-destructive">
              {error}
            </section>
          ) : null}
        </>
      ) : null}
      <section
        className={cn(
          "rounded-md border border-border bg-muted/30 p-2.5",
          compact && "border-0 bg-transparent p-0",
        )}
      >
        <div className="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
          Timeline
        </div>
        {events.length ? (
          <div className="mt-2 space-y-2">
            {[...events].reverse().map((item) => (
              <div key={item.id} className="flex gap-2">
                <span
                  className={cn(
                    "mt-1.5 size-1.5 shrink-0 rounded-full",
                    item.tone === "success"
                      ? "bg-success"
                      : item.tone === "warning"
                        ? "bg-warning"
                        : item.tone === "error"
                          ? "bg-destructive"
                          : "bg-info",
                  )}
                />
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline gap-2">
                    <span className="min-w-0 flex-1 truncate text-[10.5px] text-foreground">
                      {item.label}
                    </span>
                    <time
                      className="shrink-0 text-[9px] tabular-nums text-muted-foreground"
                      dateTime={new Date(item.createdAt).toISOString()}
                    >
                      {new Date(item.createdAt).toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </time>
                  </div>
                  {item.detail ? (
                    <div className="mt-0.5 line-clamp-2 text-[9.5px] leading-relaxed text-muted-foreground">
                      {item.detail}
                    </div>
                  ) : null}
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="mt-2 text-[10.5px] leading-relaxed text-muted-foreground">
            {hasQuery
              ? "No timeline events match this search."
              : "Run events will appear here as the agent works."}
          </p>
        )}
      </section>
    </div>
  );
}
