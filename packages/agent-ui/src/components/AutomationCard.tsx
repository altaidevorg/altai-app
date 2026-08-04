import {
  CalendarSyncIcon,
  Copy01Icon,
  Delete02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "../lib/cn.js";

export type AutomationSchedule =
  | { kind: "at"; atMs: number }
  | { kind: "every"; everyMs: number }
  | { kind: "cron"; cronExpr: string };

export type AutomationCardProps = {
  message: string;
  scheduleLabel: string;
  nextRunLabel: string;
  lastRunLabel: string;
  owningChatLabel: string;
  jobState?: string | null;
  jobError?: string | null;
  pendingRemove?: boolean;
  onOpenChat: () => void;
  onDuplicate: () => void;
  onRemove: () => void;
  className?: string;
};

export function automationScheduleLabel(schedule: AutomationSchedule): string {
  if (schedule.kind === "at") {
    return `Once · ${new Date(schedule.atMs).toLocaleString()}`;
  }
  if (schedule.kind === "every") {
    const minutes = schedule.everyMs / 60_000;
    return `Every ${minutes % 60 === 0 ? `${minutes / 60}h` : `${minutes}m`}`;
  }
  return `Cron · ${schedule.cronExpr}`;
}

export function automationLastRunLabel(lastRunAtMs: number | null): string {
  return lastRunAtMs === null
    ? "Not run yet"
    : `Last run ${new Date(lastRunAtMs).toLocaleString()}`;
}

export function automationNextRunLabel(item: {
  schedule: AutomationSchedule;
  lastRunAtMs: number | null;
}): string {
  if (item.schedule.kind === "at") {
    return `Scheduled ${new Date(item.schedule.atMs).toLocaleString()}`;
  }
  if (item.schedule.kind === "every") {
    if (item.lastRunAtMs === null) return "Next run after initial sync";
    return `Next ${new Date(item.lastRunAtMs + item.schedule.everyMs).toLocaleString()}`;
  }
  return "Next run determined by cron expression";
}

/**
 * Scheduled-automation list row. Presentational; the host owns store transport
 * and remove confirmation dialogs.
 */
export function AutomationCard({
  message,
  scheduleLabel,
  nextRunLabel,
  lastRunLabel,
  owningChatLabel,
  jobState = null,
  jobError = null,
  pendingRemove = false,
  onOpenChat,
  onDuplicate,
  onRemove,
  className,
}: AutomationCardProps) {
  const hasError = Boolean(jobError);

  return (
    <li className={cn("altai-automation-card px-3 py-3", className)}>
      <div className="flex items-start gap-2">
        <span
          className={cn(
            "mt-0.5 inline-flex size-6 shrink-0 items-center justify-center rounded-md",
            hasError
              ? "bg-destructive/10 text-destructive"
              : "bg-primary/10 text-primary",
          )}
        >
          <HugeiconsIcon icon={CalendarSyncIcon} size={13} strokeWidth={1.8} />
        </span>
        <div className="min-w-0 flex-1">
          <p className="line-clamp-3 text-[10.5px] leading-relaxed text-foreground">
            {message}
          </p>
          <span className="mt-1 inline-flex rounded bg-foreground/[0.06] px-1.5 py-0.5 text-[8.5px] font-medium text-muted-foreground">
            {scheduleLabel}
          </span>
        </div>
      </div>
      <div className="mt-2 grid grid-cols-2 gap-2 rounded-md bg-muted/50 px-2.5 py-2 text-[9.5px]">
        <div>
          <div className="text-[8.5px] font-medium uppercase tracking-wide text-muted-foreground/65">
            Next run
          </div>
          <div className="mt-0.5 text-foreground">{nextRunLabel}</div>
        </div>
        <div>
          <div className="text-[8.5px] font-medium uppercase tracking-wide text-muted-foreground/65">
            Last run
          </div>
          <div className="mt-0.5 text-muted-foreground">{lastRunLabel}</div>
        </div>
      </div>
      {jobError || jobState ? (
        <p
          className={cn(
            "mt-1 text-[9.5px]",
            hasError ? "text-destructive" : "text-muted-foreground",
          )}
        >
          {jobError ? `Failed: ${jobError}` : `Latest run: ${jobState}`}
        </p>
      ) : null}
      <div className="mt-1.5 flex items-center justify-between gap-2">
        <button
          type="button"
          onClick={onOpenChat}
          className="min-w-0 truncate text-[9.5px] text-primary hover:underline"
        >
          {owningChatLabel}
        </button>
        <button
          type="button"
          onClick={onDuplicate}
          className="ml-auto inline-flex size-6 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground"
          aria-label="Duplicate automation"
        >
          <HugeiconsIcon icon={Copy01Icon} size={11} strokeWidth={1.8} />
        </button>
        <button
          type="button"
          disabled={pendingRemove}
          onClick={onRemove}
          className="inline-flex size-6 items-center justify-center rounded-md text-muted-foreground hover:bg-destructive/10 hover:text-destructive disabled:opacity-45"
          aria-label="Remove automation"
        >
          {pendingRemove ? (
            <span
              aria-hidden
              className="inline-block size-3 animate-spin rounded-full border-2 border-muted-foreground/30 border-t-muted-foreground"
            />
          ) : (
            <HugeiconsIcon icon={Delete02Icon} size={11} strokeWidth={1.8} />
          )}
        </button>
      </div>
    </li>
  );
}
