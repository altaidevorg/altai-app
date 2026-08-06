import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";
import { InspectorMetric } from "./InspectorMetric.js";
import { SurfaceInlineError } from "./SurfaceInlineError.js";
import { SurfaceListGroup } from "./SurfaceListGroup.js";
import { SurfaceLoadingState } from "./SurfaceLoadingState.js";

export type OperationsOverviewMetric = {
  label: string;
  value: string;
};

export type OperationsOverviewRow = {
  id: string;
  title: string;
  /** Short status copy, e.g. "Working" or "Needs approval". */
  statusLabel: string;
  detail?: string;
  /** Attention rows render with destructive emphasis. */
  tone?: "default" | "attention";
  onOpen?: () => void;
  /** Optional host control (cancel run, open review, …) rendered at row end. */
  actions?: ReactNode;
};

export type OperationsOverviewProps = {
  status: "loading" | "ready" | "error";
  errorMessage?: string;
  /** Dismisses the inline error strip; host owns retry/refetch. */
  onDismissError?: () => void;
  metrics?: OperationsOverviewMetric[];
  attention?: OperationsOverviewRow[];
  progressing?: OperationsOverviewRow[];
  attentionTitle?: string;
  progressingTitle?: string;
  attentionEmptyLabel?: string;
  progressingEmptyLabel?: string;
  className?: string;
};

/**
 * Operations Overview: "what needs attention and what is progressing?".
 * Purely presentational — hosts aggregate task runs, automations, and inbox
 * notifications from their own data path (ports or control-plane projection)
 * and inject all navigation callbacks. Until a canonical `OperationsSummary`
 * projection exists, hosts compose these rows from existing domain slices.
 */
export function OperationsOverview({
  status,
  errorMessage,
  onDismissError,
  metrics = [],
  attention = [],
  progressing = [],
  attentionTitle = "Needs attention",
  progressingTitle = "In progress",
  attentionEmptyLabel = "Nothing needs attention.",
  progressingEmptyLabel = "No active work right now.",
  className,
}: OperationsOverviewProps) {
  if (status === "loading") {
    return (
      <SurfaceLoadingState className={className}>
        Loading operations…
      </SurfaceLoadingState>
    );
  }

  if (status === "error") {
    return (
      <SurfaceInlineError
        className={cn("mx-0 mt-0", className)}
        message={errorMessage ?? "Operations overview failed to load."}
        onDismiss={onDismissError}
      />
    );
  }

  return (
    <div
      aria-label="Operations overview"
      className={cn(
        "flex h-full min-h-0 flex-col gap-3 overflow-y-auto px-3 py-3",
        className,
      )}
    >
      {metrics.length ? (
        <div className="grid shrink-0 grid-cols-2 gap-px overflow-hidden rounded-lg border border-border bg-border">
          {metrics.map((metric) => (
            <InspectorMetric
              key={metric.label}
              label={metric.label}
              value={metric.value}
            />
          ))}
        </div>
      ) : null}
      <OverviewSection
        title={attentionTitle}
        rows={attention}
        emptyLabel={attentionEmptyLabel}
        ariaLabel="Operations needing attention"
      />
      <OverviewSection
        title={progressingTitle}
        rows={progressing}
        emptyLabel={progressingEmptyLabel}
        ariaLabel="Operations in progress"
      />
    </div>
  );
}

function OverviewSection({
  title,
  rows,
  emptyLabel,
  ariaLabel,
}: {
  title: string;
  rows: OperationsOverviewRow[];
  emptyLabel: string;
  ariaLabel: string;
}): ReactNode {
  return (
    <SurfaceListGroup
      title={title}
      count={rows.length}
      containerAs="ul"
      containerAriaLabel={ariaLabel}
    >
      {rows.length === 0 ? (
        <li className="px-3 py-3 text-[10.5px] text-muted-foreground">
          {emptyLabel}
        </li>
      ) : (
        rows.map((row, index) => <OverviewRow key={row.id} row={row} bordered={index > 0} />)
      )}
    </SurfaceListGroup>
  );
}

function OverviewRow({
  row,
  bordered,
}: {
  row: OperationsOverviewRow;
  bordered: boolean;
}): ReactNode {
  const status = (
    <span
      className={cn(
        "shrink-0 text-[9px] font-medium uppercase tracking-wide",
        row.tone === "attention"
          ? "text-destructive"
          : "text-muted-foreground",
      )}
    >
      {row.statusLabel}
    </span>
  );
  const text = (
    <span className="min-w-0 flex-1">
      <span className="block truncate text-[11px] font-medium text-foreground">
        {row.title}
      </span>
      {row.detail ? (
        <span className="mt-0.5 block truncate text-[9.5px] text-muted-foreground">
          {row.detail}
        </span>
      ) : null}
    </span>
  );
  const mainClassName = cn(
    "flex min-w-0 flex-1 items-center gap-2 px-3 py-2 text-left",
    row.onOpen && "transition-colors hover:bg-muted/60",
  );
  return (
    <li
      className={cn(
        "flex items-center",
        bordered && "border-t border-border-subtle",
      )}
    >
      {row.onOpen ? (
        <button type="button" onClick={row.onOpen} className={mainClassName}>
          {text}
          {status}
        </button>
      ) : (
        <div className={mainClassName}>
          {text}
          {status}
        </div>
      )}
      {row.actions ? (
        <span className="flex shrink-0 items-center gap-1 pr-2">{row.actions}</span>
      ) : null}
    </li>
  );
}
