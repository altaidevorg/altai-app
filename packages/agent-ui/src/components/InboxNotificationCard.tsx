import { cn } from "../lib/cn.js";
import { formatRelativeTime, humanize } from "../lib/inboxFormat.js";

export type InboxNotificationItem = {
  title: string;
  body: string | null;
  kind: string;
  createdAtMs: number;
  seenAtMs: number | null;
};

export type InboxNotificationCardProps = {
  notification: InboxNotificationItem;
  sessionTitle?: string;
  canOpenChat: boolean;
  busy: boolean;
  onOpenChat: () => void;
  onMarkSeen: () => void;
  onResolve: () => void;
};

/**
 * Inbox row for an agent notification. Purely presentational; the host owns
 * session lookup and mark-seen / dismiss transport.
 */
export function InboxNotificationCard({
  notification,
  sessionTitle,
  canOpenChat,
  busy,
  onOpenChat,
  onMarkSeen,
  onResolve,
}: InboxNotificationCardProps) {
  const unread = notification.seenAtMs === null;
  return (
    <article
      className={cn(
        "rounded-lg border border-border bg-muted/30 p-2.5",
        unread && "border-info/25 bg-info/[0.04]",
      )}
    >
      <div className="flex items-start gap-2">
        <span
          className={cn(
            "mt-1.5 size-1.5 shrink-0 rounded-full",
            unread ? "bg-info" : "bg-muted-foreground/35",
          )}
        />
        <div className="min-w-0 flex-1">
          <div className="flex items-start gap-2">
            <button
              type="button"
              onClick={onOpenChat}
              disabled={!canOpenChat}
              className="min-w-0 flex-1 text-left text-[11px] font-medium leading-snug text-foreground hover:underline disabled:no-underline"
            >
              {notification.title}
            </button>
            <span className="shrink-0 text-[9px] text-muted-foreground">
              {formatRelativeTime(notification.createdAtMs)}
            </span>
          </div>
          {notification.body ? (
            <p className="mt-1 whitespace-pre-wrap text-[10px] leading-relaxed text-muted-foreground">
              {notification.body}
            </p>
          ) : null}
          <div className="mt-1 flex flex-wrap items-center gap-x-1.5 text-[9px] text-muted-foreground/75">
            <span>{humanize(notification.kind)}</span>
            {sessionTitle ? (
              <>
                <span aria-hidden="true">·</span>
                <span className="max-w-40 truncate">{sessionTitle}</span>
              </>
            ) : null}
          </div>
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
        {unread ? (
          <button
            type="button"
            onClick={onMarkSeen}
            disabled={busy}
            className="rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-45"
          >
            Mark read
          </button>
        ) : null}
        <button
          type="button"
          onClick={onResolve}
          disabled={busy}
          className="ml-auto rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-45"
        >
          Dismiss
        </button>
      </div>
    </article>
  );
}
