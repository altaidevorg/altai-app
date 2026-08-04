import { Notebook01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "../lib/cn.js";
import { formatRelativeTime, humanize } from "../lib/inboxFormat.js";

export type InboxJobItem = {
  kind: string;
  state: string;
  updatedAtMs: number;
  resumeAfterRestart: boolean;
  detached: boolean;
  lastError: string | null;
};

export type InboxJobCardProps = {
  job: InboxJobItem;
  sessionTitle?: string;
  canOpenChat: boolean;
  busy: boolean;
  canDismiss: boolean;
  onOpenChat: () => void;
  onDismiss: () => void;
};

export function labelForInboxJob(job: Pick<InboxJobItem, "kind">): string {
  const kind = humanize(job.kind);
  return kind ? `${kind} background task` : "Background task";
}

/**
 * Inbox row for a background agent job. Purely presentational; the host owns
 * session lookup and dismiss transport.
 */
export function InboxJobCard({
  job,
  sessionTitle,
  canOpenChat,
  busy,
  canDismiss,
  onOpenChat,
  onDismiss,
}: InboxJobCardProps) {
  const waiting = job.state.toLowerCase().includes("waiting");
  return (
    <article className="rounded-lg border border-border bg-muted/30 p-2.5">
      <div className="flex items-start gap-2">
        <span className="mt-0.5 inline-flex size-6 shrink-0 items-center justify-center rounded-md bg-foreground/[0.06] text-muted-foreground">
          <HugeiconsIcon icon={Notebook01Icon} size={13} strokeWidth={1.75} />
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex items-start gap-2">
            <h4 className="min-w-0 flex-1 text-[11px] font-medium text-foreground">
              {labelForInboxJob(job)}
            </h4>
            <span
              className={cn(
                "rounded-full px-1.5 py-0.5 text-[9px] font-medium",
                waiting
                  ? "bg-warning/10 text-warning"
                  : "bg-info/10 text-info",
              )}
            >
              {humanize(job.state)}
            </span>
          </div>
          <p className="mt-1 text-[9.5px] text-muted-foreground">
            Updated {formatRelativeTime(job.updatedAtMs)}
            {job.resumeAfterRestart ? " · resumes after restart" : ""}
            {job.detached ? " · detached" : ""}
          </p>
          {sessionTitle ? (
            <p className="mt-0.5 truncate text-[9px] text-muted-foreground/75">
              {sessionTitle}
            </p>
          ) : null}
          {job.lastError ? (
            <p className="mt-1.5 line-clamp-2 text-[10px] leading-relaxed text-destructive">
              {job.lastError}
            </p>
          ) : null}
        </div>
      </div>
      <div className="mt-2 flex items-center gap-1 border-t border-border/40 pt-2">
        <button
          type="button"
          onClick={onOpenChat}
          disabled={!canOpenChat}
          title={
            canOpenChat
              ? "Open related chat"
              : "The related chat is unavailable until backend session recovery supports this workspace"
          }
          className="rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
        >
          Open chat
        </button>
        {canDismiss ? (
          <button
            type="button"
            onClick={onDismiss}
            disabled={busy}
            className="ml-auto rounded-md px-2 py-1 text-[10px] font-medium text-destructive hover:bg-destructive/10 disabled:opacity-45"
          >
            {busy ? "Dismissing…" : "Dismiss waiting task"}
          </button>
        ) : null}
      </div>
    </article>
  );
}
