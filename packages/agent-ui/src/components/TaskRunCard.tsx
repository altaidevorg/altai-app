import {
  ArrowReloadHorizontalIcon,
  Delete02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "../lib/cn.js";
import { TaskOutcome, type TaskOutcomeProps } from "./TaskOutcome.js";

export type TaskRunStatus =
  | "dispatching"
  | "running"
  | "awaiting-approval"
  | "done"
  | "failed"
  | "cancelled";

export type TaskRunCardProps = {
  title: string;
  status: TaskRunStatus;
  createdAtMs: number;
  tokens?: number;
  subagentCount?: number;
  agentLabel?: string;
  modelLabel?: string;
  skillsLabel?: string;
  step?: string | null;
  lastResult?: string | null;
  outcome?: TaskOutcomeProps | null;
  /** True when the task's transcript is the active chat. */
  isOpenNow?: boolean;
  /** True while the task is still running / awaiting approval. */
  active?: boolean;
  busyRetry?: boolean;
  onOpen: () => void;
  onReuse: () => void;
  onRetry?: () => void;
  onStop?: () => void;
  onRemove?: () => void;
  /** Injected clock for stable relative-age labels in tests. */
  nowMs?: number;
  className?: string;
};

const STATUS_COPY: Record<TaskRunStatus, string> = {
  dispatching: "Starting",
  running: "Working",
  "awaiting-approval": "Needs approval",
  done: "Done",
  failed: "Failed",
  cancelled: "Stopped",
};

export function formatTaskAge(
  createdAtMs: number,
  nowMs: number = Date.now(),
): string {
  const minutes = Math.max(0, Math.floor((nowMs - createdAtMs) / 60_000));
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

function formatTokens(tokens: number): string {
  return tokens >= 1000 ? `${(tokens / 1000).toFixed(1)}k` : String(tokens);
}

/**
 * Background work queue card. Presentational; the host owns assignment/run
 * stores and open/reuse/retry/stop/remove transport.
 */
export function TaskRunCard({
  title,
  status,
  createdAtMs,
  tokens = 0,
  subagentCount = 0,
  agentLabel,
  modelLabel,
  skillsLabel,
  step = null,
  lastResult = null,
  outcome = null,
  isOpenNow = false,
  active = false,
  busyRetry = false,
  onOpen,
  onReuse,
  onRetry,
  onStop,
  onRemove,
  nowMs,
  className,
}: TaskRunCardProps) {
  const meta = [
    STATUS_COPY[status],
    tokens ? `${formatTokens(tokens)} tokens` : null,
    subagentCount ? `${subagentCount} agents` : null,
    agentLabel,
    modelLabel,
    skillsLabel,
  ].filter(Boolean);

  return (
    <article className={cn("altai-task-run-card p-3", className)}>
      <div className="flex items-start gap-2">
        <span
          className={cn(
            "mt-1.5 size-1.5 shrink-0 rounded-full",
            status === "failed"
              ? "bg-destructive"
              : status === "done"
                ? "bg-success"
                : status === "cancelled"
                  ? "bg-muted-foreground/50"
                  : "animate-pulse bg-info",
          )}
        />
        <div className="min-w-0 flex-1">
          <button
            type="button"
            onClick={onOpen}
            className="line-clamp-2 text-left text-[11.5px] font-medium leading-snug text-foreground hover:underline"
          >
            {title}
          </button>
          <p className="mt-1 text-[10px] text-muted-foreground">
            <span
              className={cn(
                status === "failed" && "text-destructive",
                status === "done" && "text-success",
              )}
            >
              {meta[0]}
            </span>
            {meta.slice(1).map((part) => (
              <span key={part}>{` · ${part}`}</span>
            ))}
          </p>
        </div>
        <time
          dateTime={new Date(createdAtMs).toISOString()}
          className="shrink-0 text-[9px] tabular-nums text-muted-foreground/70"
        >
          {formatTaskAge(createdAtMs, nowMs)}
        </time>
      </div>

      {active && step ? (
        <p className="mt-2 flex items-center gap-1.5 truncate rounded-md bg-muted/70 px-2 py-1.5 text-[10px] text-muted-foreground">
          <span
            aria-hidden
            className="inline-block size-3 shrink-0 animate-spin rounded-full border-2 border-muted-foreground/30 border-t-muted-foreground"
          />
          {step}
        </p>
      ) : null}

      {status === "done" && lastResult ? (
        <p className="mt-2 line-clamp-2 text-[10px] leading-relaxed text-muted-foreground">
          {lastResult}
        </p>
      ) : null}

      {(status === "done" || status === "failed") && outcome ? (
        <TaskOutcome {...outcome} />
      ) : null}

      <div className="mt-2 flex items-center gap-1">
        <button
          type="button"
          onClick={onOpen}
          className="rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          {isOpenNow ? "Open now" : "Open transcript"}
        </button>
        <button
          type="button"
          onClick={onReuse}
          className="rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          Reuse
        </button>
        {status === "failed" && onRetry ? (
          <button
            type="button"
            disabled={busyRetry}
            onClick={onRetry}
            className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-[10px] font-medium text-foreground hover:bg-muted disabled:opacity-45"
          >
            <HugeiconsIcon
              icon={ArrowReloadHorizontalIcon}
              size={10}
              strokeWidth={2}
            />
            Retry
          </button>
        ) : null}
        {active && onStop ? (
          <button
            type="button"
            onClick={onStop}
            className="ml-auto rounded-md px-2 py-1 text-[10px] font-medium text-destructive hover:bg-destructive/10"
          >
            Stop
          </button>
        ) : onRemove ? (
          <button
            type="button"
            onClick={onRemove}
            aria-label={`Remove ${title}`}
            className="ml-auto inline-flex size-6 items-center justify-center rounded-md text-muted-foreground/70 hover:bg-destructive/10 hover:text-destructive"
          >
            <HugeiconsIcon icon={Delete02Icon} size={11} strokeWidth={1.8} />
          </button>
        ) : null}
      </div>
    </article>
  );
}
