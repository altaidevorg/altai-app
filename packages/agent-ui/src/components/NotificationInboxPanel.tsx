import {
  Alert02Icon,
  Cancel01Icon,
  Notification01Icon,
  Refresh01Icon,
  TickDouble01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  AuxiliarySurface,
  SurfaceIconAction,
  SurfaceSearch,
  SurfaceTabs,
} from "./AuxiliarySurface.js";
import { EmptyInbox } from "./EmptyInbox.js";
import { FilteredEmptyInbox } from "./FilteredEmptyInbox.js";
import {
  InboxJobCard,
  type InboxJobItem,
} from "./InboxJobCard.js";
import { InboxLoadFailed } from "./InboxLoadFailed.js";
import {
  InboxNotificationCard,
  type InboxNotificationItem,
} from "./InboxNotificationCard.js";
import { InboxSection } from "./InboxSection.js";
import {
  InboxTicketCard,
  type InboxTicketItem,
} from "./InboxTicketCard.js";

export type NotificationInboxFilter = "all" | "attention" | "updates";

export type NotificationInboxTicketRow = {
  id: string;
  ticket: InboxTicketItem;
  sessionTitle?: string;
  busy?: boolean;
  canOpenChat?: boolean;
  canResume?: boolean;
  canDismiss?: boolean;
};

export type NotificationInboxJobRow = {
  id: string;
  job: InboxJobItem;
  sessionTitle?: string;
  busy?: boolean;
  canOpenChat?: boolean;
  canDismiss?: boolean;
};

export type NotificationInboxNotificationRow = {
  id: string;
  notification: InboxNotificationItem;
  sessionTitle?: string;
  busy?: boolean;
  canOpenChat?: boolean;
};

export type NotificationInboxPanelProps = {
  attentionCount: number;
  filter: NotificationInboxFilter;
  onFilterChange: (filter: NotificationInboxFilter) => void;
  filterCounts: Record<NotificationInboxFilter, number>;
  query: string;
  onQueryChange: (query: string) => void;
  error?: string | null;
  onDismissError?: () => void;
  loading?: boolean;
  hydrated?: boolean;
  empty?: boolean;
  hasVisibleItems?: boolean;
  markingAllRead?: boolean;
  unreadCount?: number;
  onMarkAllRead?: () => void;
  onRefresh?: () => void;
  onClose?: () => void;
  onRetry?: () => void;
  /** Fill parent shell (Operations) instead of absolute AI overlay. */
  presentation?: "overlay" | "embedded";
  tickets?: NotificationInboxTicketRow[];
  jobs?: NotificationInboxJobRow[];
  unreadNotifications?: NotificationInboxNotificationRow[];
  readNotifications?: NotificationInboxNotificationRow[];
  allNotifications?: NotificationInboxNotificationRow[];
  onOpenTicketChat?: (id: string) => void;
  onReplyTicket?: (id: string, response: string) => void;
  onDismissTicket?: (id: string) => void;
  onOpenJobChat?: (id: string) => void;
  onDismissJob?: (id: string) => void;
  onOpenNotificationChat?: (id: string) => void;
  onMarkNotificationSeen?: (id: string) => void;
  onResolveNotification?: (id: string) => void;
};

function LoadingDots() {
  return (
    <span
      aria-hidden
      className="inline-block size-3.5 animate-spin rounded-full border-2 border-muted-foreground/30 border-t-muted-foreground"
    />
  );
}

/**
 * Agent inbox surface: search/filter chrome, empty/error/loading states, and
 * ticket/job/notification sections. Host owns store transport and dismiss
 * confirmation dialogs.
 */
export function NotificationInboxPanel({
  attentionCount,
  filter,
  onFilterChange,
  filterCounts,
  query,
  onQueryChange,
  error = null,
  onDismissError,
  loading = false,
  hydrated = true,
  empty = false,
  hasVisibleItems = false,
  markingAllRead = false,
  unreadCount = 0,
  onMarkAllRead,
  onRefresh,
  onClose,
  onRetry,
  presentation = "overlay",
  tickets = [],
  jobs = [],
  unreadNotifications = [],
  readNotifications = [],
  allNotifications = [],
  onOpenTicketChat,
  onReplyTicket,
  onDismissTicket,
  onOpenJobChat,
  onDismissJob,
  onOpenNotificationChat,
  onMarkNotificationSeen,
  onResolveNotification,
}: NotificationInboxPanelProps) {
  const showTickets = filter === "all" || filter === "attention";
  const showJobs = filter === "all" || filter === "attention";
  const showUnreadOrAll =
    filter === "all" || filter === "attention" || filter === "updates";
  const primaryNotifications =
    filter === "updates" ? allNotifications : unreadNotifications;

  return (
    <AuxiliarySurface
      title="Inbox"
      eyebrow="Agent attention"
      icon={Notification01Icon}
      presentation={presentation}
      subtitle={
        attentionCount
          ? `${attentionCount} item${attentionCount === 1 ? "" : "s"} need your attention`
          : "Nothing is blocking your agents"
      }
      status={
        attentionCount ? (
          <span className="rounded bg-warning/12 px-1.5 py-0.5 text-[8.5px] font-semibold text-warning">
            Action needed
          </span>
        ) : (
          <span className="rounded bg-success/10 px-1.5 py-0.5 text-[8.5px] font-semibold text-success">
            All clear
          </span>
        )
      }
      onClose={onClose}
      actions={
        <>
          <SurfaceIconAction
            label="Mark every notification as read"
            onClick={() => onMarkAllRead?.()}
            disabled={!unreadCount || markingAllRead || !onMarkAllRead}
          >
            {markingAllRead ? (
              <LoadingDots />
            ) : (
              <HugeiconsIcon
                icon={TickDouble01Icon}
                size={13}
                strokeWidth={1.75}
              />
            )}
          </SurfaceIconAction>
          <SurfaceIconAction
            label="Refresh agent inbox"
            onClick={() => onRefresh?.()}
            disabled={loading || !onRefresh}
          >
            {loading ? (
              <LoadingDots />
            ) : (
              <HugeiconsIcon icon={Refresh01Icon} size={13} strokeWidth={1.75} />
            )}
          </SurfaceIconAction>
        </>
      }
    >
      {error ? (
        <div
          role="alert"
          className="mx-3 mt-3 flex items-start gap-2 rounded-none border border-destructive/30 bg-destructive/[0.06] px-2.5 py-2 text-[10.5px] text-destructive"
        >
          <HugeiconsIcon
            icon={Alert02Icon}
            size={13}
            strokeWidth={1.8}
            className="mt-0.5 shrink-0"
          />
          <span className="min-w-0 flex-1">{error}</span>
          {onDismissError ? (
            <button
              type="button"
              onClick={onDismissError}
              aria-label="Dismiss error"
              className="rounded p-0.5 hover:bg-destructive/10"
            >
              <HugeiconsIcon icon={Cancel01Icon} size={11} strokeWidth={2} />
            </button>
          ) : null}
        </div>
      ) : null}

      <div className="shrink-0 space-y-2 border-b border-border-subtle bg-card px-3 py-2.5">
        <SurfaceSearch
          value={query}
          onChange={onQueryChange}
          placeholder="Search by update, task, or conversation"
          className="w-full"
        />
        <SurfaceTabs
          label="Filter inbox"
          value={filter}
          onChange={(value) => onFilterChange(value as NotificationInboxFilter)}
          items={[
            { id: "all", label: "All", count: filterCounts.all },
            {
              id: "attention",
              label: "Attention",
              count: filterCounts.attention,
            },
            {
              id: "updates",
              label: "Updates",
              count: filterCounts.updates,
            },
          ]}
          className="border-0 bg-transparent p-0"
        />
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-3">
        {!hydrated && loading ? (
          <div className="flex items-center justify-center gap-2 py-10 text-[11px] text-muted-foreground">
            <LoadingDots />
            Loading agent inbox…
          </div>
        ) : error ? (
          <InboxLoadFailed onRetry={() => onRetry?.()} />
        ) : empty ? (
          <EmptyInbox />
        ) : !hasVisibleItems ? (
          <FilteredEmptyInbox
            label={
              filter === "attention"
                ? "Nothing needs your attention"
                : "No updates to show"
            }
            onShowAll={() => onFilterChange("all")}
          />
        ) : (
          <div className="space-y-4">
            {showTickets && tickets.length ? (
              <InboxSection title="Paused tasks" count={tickets.length}>
                {tickets.map((row) => (
                  <InboxTicketCard
                    key={row.id}
                    ticket={row.ticket}
                    sessionTitle={row.sessionTitle}
                    busy={Boolean(row.busy)}
                    canOpenChat={Boolean(row.canOpenChat)}
                    canResume={Boolean(row.canResume)}
                    canDismiss={Boolean(row.canDismiss)}
                    onReply={(response) => onReplyTicket?.(row.id, response)}
                    onOpenChat={() => onOpenTicketChat?.(row.id)}
                    onDismiss={() => onDismissTicket?.(row.id)}
                  />
                ))}
              </InboxSection>
            ) : null}

            {showJobs && jobs.length ? (
              <InboxSection title="Waiting work" count={jobs.length}>
                {jobs.map((row) => (
                  <InboxJobCard
                    key={row.id}
                    job={row.job}
                    sessionTitle={row.sessionTitle}
                    canOpenChat={Boolean(row.canOpenChat)}
                    busy={Boolean(row.busy)}
                    canDismiss={Boolean(row.canDismiss)}
                    onOpenChat={() => onOpenJobChat?.(row.id)}
                    onDismiss={() => onDismissJob?.(row.id)}
                  />
                ))}
              </InboxSection>
            ) : null}

            {showUnreadOrAll && primaryNotifications.length ? (
              <InboxSection
                title={filter === "updates" ? "Updates" : "Unread updates"}
                count={primaryNotifications.length}
              >
                {primaryNotifications.map((row) => (
                  <InboxNotificationCard
                    key={row.id}
                    notification={row.notification}
                    sessionTitle={row.sessionTitle}
                    canOpenChat={Boolean(row.canOpenChat)}
                    busy={Boolean(row.busy)}
                    onOpenChat={() => onOpenNotificationChat?.(row.id)}
                    onMarkSeen={() => onMarkNotificationSeen?.(row.id)}
                    onResolve={() => onResolveNotification?.(row.id)}
                  />
                ))}
              </InboxSection>
            ) : null}

            {filter === "all" && readNotifications.length ? (
              <InboxSection
                title="Earlier updates"
                count={readNotifications.length}
              >
                {readNotifications.map((row) => (
                  <InboxNotificationCard
                    key={row.id}
                    notification={row.notification}
                    sessionTitle={row.sessionTitle}
                    canOpenChat={Boolean(row.canOpenChat)}
                    busy={Boolean(row.busy)}
                    onOpenChat={() => onOpenNotificationChat?.(row.id)}
                    onMarkSeen={() => onMarkNotificationSeen?.(row.id)}
                    onResolve={() => onResolveNotification?.(row.id)}
                  />
                ))}
              </InboxSection>
            ) : null}
          </div>
        )}
      </div>
    </AuxiliarySurface>
  );
}
