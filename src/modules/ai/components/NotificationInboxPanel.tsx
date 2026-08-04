import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Spinner } from "@/components/ui/spinner";
import { useWorkspaceFolderStore } from "@/modules/workspace/folder";
import {
  Alert02Icon,
  Cancel01Icon,
  Notification01Icon,
  Refresh01Icon,
  TickDouble01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useMemo, useState } from "react";
import type { AgentNotificationInfo } from "../lib/native";
import { useChatStore } from "../store/chatStore";
import {
  buildNotificationInboxView,
  useNotificationStore,
} from "../store/notificationStore";
import {
  AuxiliarySurface,
  EmptyInbox,
  FilteredEmptyInbox,
  InboxJobCard,
  InboxLoadFailed,
  InboxNotificationCard,
  InboxSection,
  InboxTicketCard,
  labelForInboxJob,
  SurfaceIconAction,
  SurfaceSearch,
  SurfaceTabs,
} from "@altai/agent-ui";

type DismissTarget =
  | {
      kind: "ticket";
      id: string;
      chatId: string;
      label: string;
    }
  | {
      kind: "job";
      id: string;
      chatId: string;
      label: string;
    };

type InboxFilter = "all" | "attention" | "updates";

export function NotificationInboxPanel({ onClose }: { onClose: () => void }) {
  const workspacePath = useWorkspaceFolderStore((state) => state.folder);
  const sessions = useChatStore((state) => state.sessions);
  const switchSession = useChatStore((state) => state.switchSession);
  const notifications = useNotificationStore((state) => state.notifications);
  const backgroundJobs = useNotificationStore((state) => state.backgroundJobs);
  const clarificationTickets = useNotificationStore(
    (state) => state.clarificationTickets,
  );
  const hydrated = useNotificationStore((state) => state.hydrated);
  const loading = useNotificationStore((state) => state.loading);
  const error = useNotificationStore((state) => state.error);
  const pendingIds = useNotificationStore((state) => state.pendingIds);
  const refresh = useNotificationStore((state) => state.refresh);
  const markSeen = useNotificationStore((state) => state.markSeen);
  const resolveNotification = useNotificationStore(
    (state) => state.resolveNotification,
  );
  const dismissJob = useNotificationStore((state) => state.dismissJob);
  const dismissTicket = useNotificationStore((state) => state.dismissTicket);
  const replyToTicket = useNotificationStore((state) => state.replyToTicket);
  const clearError = useNotificationStore((state) => state.clearError);
  const [dismissTarget, setDismissTarget] = useState<DismissTarget | null>(null);
  const [filter, setFilter] = useState<InboxFilter>("all");
  const [query, setQuery] = useState("");
  const [markingAllRead, setMarkingAllRead] = useState(false);

  const view = useMemo(
    () =>
      buildNotificationInboxView(
        notifications,
        backgroundJobs,
        clarificationTickets,
      ),
    [notifications, backgroundJobs, clarificationTickets],
  );
  const sessionIds = useMemo(
    () => new Set(sessions.map((session) => session.id)),
    [sessions],
  );
  const sessionTitles = useMemo(
    () => new Map(sessions.map((session) => [session.id, session.title])),
    [sessions],
  );
  useEffect(() => {
    void refresh(workspacePath);
  }, [refresh, workspacePath]);

  const openChat = (chatId: string) => {
    if (!sessionIds.has(chatId)) return;
    switchSession(chatId);
    onClose();
  };

  const openNotificationChat = (notification: AgentNotificationInfo) => {
    if (notification.seenAtMs === null) {
      void markSeen(notification.id, notification.chatId);
    }
    openChat(notification.chatId);
  };

  const confirmDismiss = () => {
    const target = dismissTarget;
    if (!target) return;
    if (target.kind === "ticket") {
      void dismissTicket(target.id, target.chatId);
    } else {
      void dismissJob(target.id, target.chatId);
    }
  };

  const empty =
    view.waitingTickets.length === 0 &&
    view.notifications.length === 0 &&
    view.waitingJobs.length === 0;
  const unreadNotifications = useMemo(
    () => view.notifications.filter((notification) => notification.seenAtMs === null),
    [view.notifications],
  );
  const matchesQuery = (values: Array<string | null | undefined>) => {
    const normalizedQuery = query.trim().toLowerCase();
    if (!normalizedQuery) return true;
    return values
      .filter(Boolean)
      .join("\n")
      .toLowerCase()
      .includes(normalizedQuery);
  };
  const visibleTickets = useMemo(
    () =>
      view.waitingTickets.filter((ticket) =>
        matchesQuery([
          ticket.prompt,
          ...ticket.choices,
          sessionTitles.get(ticket.chatId),
        ]),
      ),
    [query, sessionTitles, view.waitingTickets],
  );
  const visibleNotifications = useMemo(
    () =>
      (filter === "attention" ? unreadNotifications : view.notifications).filter(
        (notification) =>
          matchesQuery([
            notification.title,
            notification.body,
            notification.kind,
            sessionTitles.get(notification.chatId),
          ]),
      ),
    [filter, query, sessionTitles, unreadNotifications, view.notifications],
  );
  const visibleUnreadNotifications = visibleNotifications.filter(
    (notification) => notification.seenAtMs === null,
  );
  const visibleReadNotifications = visibleNotifications.filter(
    (notification) => notification.seenAtMs !== null,
  );
  const visibleWaitingJobs = useMemo(
    () =>
      view.waitingJobs.filter((job) =>
        matchesQuery([
          job.kind,
          job.state,
          job.lastError,
          sessionTitles.get(job.chatId),
        ]),
      ),
    [query, sessionTitles, view.waitingJobs],
  );
  const filterCounts: Record<InboxFilter, number> = {
    all:
      view.waitingTickets.length +
      view.notifications.length +
      view.waitingJobs.length,
    attention:
      view.waitingTickets.length + unreadNotifications.length + view.waitingJobs.length,
    updates: view.notifications.length,
  };
  const hasVisibleItems =
    ((filter === "all" || filter === "attention") &&
      visibleTickets.length > 0) ||
    ((filter === "all" || filter === "attention" || filter === "updates") &&
      visibleNotifications.length > 0) ||
    ((filter === "all" || filter === "attention") &&
      visibleWaitingJobs.length > 0);

  const markAllRead = async () => {
    if (!unreadNotifications.length || markingAllRead) return;
    setMarkingAllRead(true);
    try {
      await Promise.all(
        unreadNotifications.map((notification) =>
          markSeen(notification.id, notification.chatId),
        ),
      );
    } finally {
      setMarkingAllRead(false);
    }
  };

  return (
    <>
      <AuxiliarySurface
        title="Inbox"
        eyebrow="Agent attention"
        icon={Notification01Icon}
        subtitle={
          view.attentionCount
            ? `${view.attentionCount} item${view.attentionCount === 1 ? "" : "s"} need your attention`
            : "Nothing is blocking your agents"
        }
        status={
          view.attentionCount ? (
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
              onClick={() => void markAllRead()}
              disabled={!unreadNotifications.length || markingAllRead}
            >
              {markingAllRead ? (
                <Spinner className="size-3.5" />
              ) : (
                <HugeiconsIcon icon={TickDouble01Icon} size={13} strokeWidth={1.75} />
              )}
            </SurfaceIconAction>
            <SurfaceIconAction
              label="Refresh agent inbox"
              onClick={() => void refresh(workspacePath)}
              disabled={loading}
            >
              {loading ? (
                <Spinner className="size-3.5" />
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
            <button
              type="button"
              onClick={clearError}
              aria-label="Dismiss error"
              className="rounded p-0.5 hover:bg-destructive/10"
            >
              <HugeiconsIcon icon={Cancel01Icon} size={11} strokeWidth={2} />
            </button>
          </div>
        ) : null}

        <div className="shrink-0 space-y-2 border-b border-border-subtle bg-card px-3 py-2.5">
            <SurfaceSearch
              value={query}
              onChange={setQuery}
              placeholder="Search by update, task, or conversation"
              className="w-full"
            />
            <SurfaceTabs
              label="Filter inbox"
              value={filter}
              onChange={(value) => setFilter(value as InboxFilter)}
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
              <Spinner className="size-3.5" />
              Loading agent inbox…
            </div>
          ) : error ? (
            <InboxLoadFailed onRetry={() => void refresh(workspacePath)} />
          ) : empty ? (
            <EmptyInbox />
          ) : !hasVisibleItems ? (
            <FilteredEmptyInbox
              label={filter === "attention" ? "Nothing needs your attention" : "No updates to show"}
              onShowAll={() => setFilter("all")}
            />
          ) : (
            <div className="space-y-4">
              {(filter === "all" || filter === "attention") &&
              visibleTickets.length ? (
                <InboxSection
                  title="Paused tasks"
                  count={visibleTickets.length}
                >
                  {visibleTickets.map((ticket) => (
                    <InboxTicketCard
                      key={ticket.id}
                      ticket={ticket}
                      sessionTitle={sessionTitles.get(ticket.chatId)}
                      busy={Boolean(pendingIds[`ticket:${ticket.id}`])}
                      canOpenChat={sessionIds.has(ticket.chatId)}
                      canResume={backgroundJobs.some(
                        (job) =>
                          job.id === ticket.jobId &&
                          job.state.trim().toLowerCase() === "waiting",
                      )}
                      canDismiss={backgroundJobs.some(
                        (job) =>
                          job.id === ticket.jobId &&
                          job.state.trim().toLowerCase() === "waiting",
                      )}
                      onReply={(response) =>
                        void replyToTicket(ticket.id, ticket.chatId, response)
                      }
                      onOpenChat={() => openChat(ticket.chatId)}
                      onDismiss={() =>
                        setDismissTarget({
                          kind: "ticket",
                          id: ticket.id,
                          chatId: ticket.chatId,
                          label: ticket.prompt,
                        })
                      }
                    />
                  ))}
                </InboxSection>
              ) : null}

              {(filter === "all" || filter === "attention") &&
              visibleWaitingJobs.length ? (
                <InboxSection title="Waiting work" count={visibleWaitingJobs.length}>
                  {visibleWaitingJobs.map((job) => (
                    <InboxJobCard
                      key={job.id}
                      job={job}
                      sessionTitle={sessionTitles.get(job.chatId)}
                      canOpenChat={sessionIds.has(job.chatId)}
                      busy={Boolean(pendingIds[`job:${job.id}`])}
                      canDismiss
                      onOpenChat={() => openChat(job.chatId)}
                      onDismiss={() =>
                        setDismissTarget({
                          kind: "job",
                          id: job.id,
                          chatId: job.chatId,
                          label: labelForInboxJob(job),
                        })
                      }
                    />
                  ))}
                </InboxSection>
              ) : null}

              {(filter === "all" ||
                filter === "updates" ||
                filter === "attention") &&
              (filter === "updates"
                ? visibleNotifications.length
                : visibleUnreadNotifications.length) ? (
                <InboxSection
                  title={filter === "updates" ? "Updates" : "Unread updates"}
                  count={
                    filter === "updates"
                      ? visibleNotifications.length
                      : visibleUnreadNotifications.length
                  }
                >
                  {(filter === "updates"
                    ? visibleNotifications
                    : visibleUnreadNotifications
                  ).map((notification) => (
                    <InboxNotificationCard
                      key={notification.id}
                      notification={notification}
                      sessionTitle={sessionTitles.get(notification.chatId)}
                      canOpenChat={sessionIds.has(notification.chatId)}
                      busy={Boolean(
                        pendingIds[`notification:${notification.id}`],
                      )}
                      onOpenChat={() => openNotificationChat(notification)}
                      onMarkSeen={() =>
                        void markSeen(notification.id, notification.chatId)
                      }
                      onResolve={() =>
                        void resolveNotification(
                          notification.id,
                          notification.chatId,
                        )
                      }
                    />
                  ))}
                </InboxSection>
              ) : null}

              {filter === "all" && visibleReadNotifications.length ? (
                <InboxSection title="Earlier updates" count={visibleReadNotifications.length}>
                  {visibleReadNotifications.map((notification) => (
                    <InboxNotificationCard
                      key={notification.id}
                      notification={notification}
                      sessionTitle={sessionTitles.get(notification.chatId)}
                      canOpenChat={sessionIds.has(notification.chatId)}
                      busy={Boolean(
                        pendingIds[`notification:${notification.id}`],
                      )}
                      onOpenChat={() => openNotificationChat(notification)}
                      onMarkSeen={() =>
                        void markSeen(notification.id, notification.chatId)
                      }
                      onResolve={() =>
                        void resolveNotification(
                          notification.id,
                          notification.chatId,
                        )
                      }
                    />
                  ))}
                </InboxSection>
              ) : null}
            </div>
          )}
        </div>
      </AuxiliarySurface>

      <AlertDialog
        open={dismissTarget !== null}
        onOpenChange={(open) => {
          if (!open) setDismissTarget(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Dismiss background task?</AlertDialogTitle>
            <AlertDialogDescription>
              {dismissTarget?.kind === "ticket"
                ? "This marks the waiting background job as completed and dismisses every unanswered question attached to it."
                : "This marks the waiting background job as completed and dismisses its outstanding questions."}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="line-clamp-3 rounded-lg bg-muted/50 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground">
            {dismissTarget?.label}
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep task</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={confirmDismiss}>
              Dismiss task
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}



