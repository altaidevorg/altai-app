import { Alert02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";
import { cn } from "../lib/cn.js";
import { formatRelativeTime } from "../lib/inboxFormat.js";

export type InboxTicketItem = {
  prompt: string;
  choices: string[];
  updatedAtMs: number;
};

export type InboxTicketCardProps = {
  ticket: InboxTicketItem;
  sessionTitle?: string;
  busy: boolean;
  canOpenChat: boolean;
  canResume: boolean;
  canDismiss: boolean;
  onOpenChat: () => void;
  onReply: (response: string) => void;
  onDismiss: () => void;
};

/**
 * Inbox row for a clarification ticket that paused a background task.
 * Owns local reply draft state; the host supplies resume/dismiss transport.
 */
export function InboxTicketCard({
  ticket,
  sessionTitle,
  busy,
  canOpenChat,
  canResume,
  canDismiss,
  onOpenChat,
  onReply,
  onDismiss,
}: InboxTicketCardProps) {
  const [response, setResponse] = useState("");
  const trimmedResponse = response.trim();

  return (
    <article className="rounded-lg border border-warning/30 bg-warning/[0.06] p-2.5">
      <div className="flex items-start gap-2">
        <span className="mt-0.5 inline-flex size-6 shrink-0 items-center justify-center rounded-md bg-warning/10 text-warning">
          <HugeiconsIcon icon={Alert02Icon} size={13} strokeWidth={1.8} />
        </span>
        <div className="min-w-0 flex-1">
          <div className="text-[10px] font-medium uppercase tracking-wide text-warning/80">
            Background task is paused
          </div>
          {sessionTitle ? (
            <button
              type="button"
              onClick={onOpenChat}
              disabled={!canOpenChat}
              className="mt-0.5 max-w-full truncate text-left text-[9.5px] text-muted-foreground hover:text-foreground disabled:opacity-45"
            >
              {sessionTitle}
            </button>
          ) : null}
          <p className="mt-1 whitespace-pre-wrap text-[11px] leading-relaxed text-foreground">
            {ticket.prompt}
          </p>
          {!canResume && ticket.choices.length ? (
            <div
              aria-label="Available choices"
              className="mt-2 flex flex-wrap gap-1"
            >
              {ticket.choices.map((choice, index) => (
                <span
                  key={`${index}-${choice}`}
                  className="border border-warning/25 bg-muted px-2 py-0.5 text-[9.5px] text-muted-foreground"
                >
                  {choice}
                </span>
              ))}
            </div>
          ) : null}
          <div className="mt-1.5 text-[9px] text-muted-foreground">
            {formatRelativeTime(ticket.updatedAtMs)}
          </div>
          {canResume ? (
            <div className="mt-2 space-y-1.5">
              <textarea
                value={response}
                onChange={(event) => setResponse(event.target.value)}
                disabled={busy}
                placeholder="Reply to resume this task…"
                aria-label="Response to clarification ticket"
                rows={2}
                maxLength={10_000}
                className="w-full resize-y border border-warning/25 bg-muted px-2 py-1.5 text-[10.5px] leading-relaxed outline-none placeholder:text-muted-foreground/70 focus:border-warning/55 disabled:opacity-50"
              />
              {ticket.choices.length ? (
                <div className="flex flex-wrap gap-1">
                  {ticket.choices.map((choice, index) => (
                    <button
                      key={`${index}-${choice}-reply`}
                      type="button"
                      onClick={() => setResponse(choice)}
                      disabled={busy}
                      className={cn(
                        "border px-2 py-0.5 text-[9px] transition-colors disabled:opacity-45",
                        response === choice
                          ? "border-warning/60 bg-warning/15 text-warning"
                          : "border-warning/25 bg-muted text-muted-foreground hover:border-warning/45",
                      )}
                    >
                      {choice}
                    </button>
                  ))}
                </div>
              ) : null}
            </div>
          ) : (
            <p className="mt-1 text-[9.5px] leading-relaxed text-muted-foreground">
              This task is no longer waiting for a reply.
            </p>
          )}
        </div>
      </div>
      <div className="mt-2 flex items-center gap-1 border-t border-warning/15 pt-2">
        {canResume ? (
          <button
            type="button"
            onClick={() => onReply(trimmedResponse)}
            disabled={busy || !trimmedResponse}
            className="rounded-md bg-warning/15 px-2 py-1 text-[10px] font-medium text-warning transition-colors hover:bg-warning/25 disabled:cursor-not-allowed disabled:opacity-45"
          >
            {busy ? "Resuming…" : "Reply & resume"}
          </button>
        ) : null}
        {canDismiss ? (
          <button
            type="button"
            onClick={onDismiss}
            disabled={busy}
            className="ml-auto rounded-md px-2 py-1 text-[10px] font-medium text-destructive hover:bg-destructive/10 disabled:opacity-45"
          >
            {busy ? "Dismissing…" : "Dismiss waiting task"}
          </button>
        ) : (
          <span className="text-[9.5px] text-muted-foreground">
            Waiting for safe resume routing
          </span>
        )}
      </div>
    </article>
  );
}
