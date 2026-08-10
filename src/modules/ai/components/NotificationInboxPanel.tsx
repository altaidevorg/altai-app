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
import { useWorkspaceFolderStore } from "@/modules/workspace/folder";
import { useEffect, useMemo, useState } from "react";
import {
  labelForInboxJob,
  NotificationInboxPanel as SharedNotificationInboxPanel,
  notificationInboxFilterCounts,
  notificationInboxHasVisibleItems,
  notificationsForInboxFilter,
  partitionNotificationsByReadState,
  type NotificationInboxFilter,
  sessionIdSet,
  sessionTitleMap,
  isWaitingTicketStatus,
  filterUnreadBySeenAtMs,
  mapById,
  isNotificationInboxEmpty,
  filterRowsBySearchFields,
} from "@altai/agent-ui";
import type { AgentNotificationInfo } from "../lib/native";
import { useChatStore } from "../store/chatStore";
import {
  buildNotificationInboxView,
  useNotificationStore,
} from "../store/notificationStore";

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

export function NotificationInboxPanel({
  onClose,
  presentation = "overlay",
}: {
  onClose?: () => void;
  presentation?: "overlay" | "embedded";
}) {
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
  const [filter, setFilter] = useState<NotificationInboxFilter>("all");
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
    () => sessionIdSet(sessions),
    [sessions],
  );
  const sessionTitles = useMemo(
    () => sessionTitleMap(sessions),
    [sessions],
  );
  useEffect(() => {
    void refresh(workspacePath);
  }, [refresh, workspacePath]);

  const openChat = (chatId: string) => {
    if (!sessionIds.has(chatId)) return;
    switchSession(chatId);
    onClose?.();
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

  const empty = isNotificationInboxEmpty({
    waitingTickets: view.waitingTickets.length,
    notifications: view.notifications.length,
    waitingJobs: view.waitingJobs.length,
  });
  const unreadNotifications = useMemo(
    () => filterUnreadBySeenAtMs(view.notifications),
    [view.notifications],
  );
  const visibleTickets = useMemo(
    () =>
      filterRowsBySearchFields(
        view.waitingTickets,
        query,
        (ticket) => [
          ticket.prompt,
          ...ticket.choices,
          sessionTitles.get(ticket.chatId),
        ],
      ),
    [query, sessionTitles, view.waitingTickets],
  );
  const visibleNotifications = useMemo(
    () =>
      filterRowsBySearchFields(
        notificationsForInboxFilter(
          filter,
          view.notifications,
          unreadNotifications,
        ),
        query,
        (notification) => [
          notification.title,
          notification.body,
          notification.kind,
          sessionTitles.get(notification.chatId),
        ],
      ),
    [filter, query, sessionTitles, unreadNotifications, view.notifications],
  );
  const { unread: visibleUnreadNotifications, read: visibleReadNotifications } =
    useMemo(
      () => partitionNotificationsByReadState(visibleNotifications),
      [visibleNotifications],
    );
  const visibleWaitingJobs = useMemo(
    () =>
      filterRowsBySearchFields(
        view.waitingJobs,
        query,
        (job) => [
          job.kind,
          job.state,
          job.lastError,
          sessionTitles.get(job.chatId),
        ],
      ),
    [query, sessionTitles, view.waitingJobs],
  );
  const filterCounts: Record<NotificationInboxFilter, number> =
    notificationInboxFilterCounts({
      waitingTickets: view.waitingTickets.length,
      notifications: view.notifications.length,
      waitingJobs: view.waitingJobs.length,
      unreadNotifications: unreadNotifications.length,
    });
  const hasVisibleItems = notificationInboxHasVisibleItems(filter, {
    tickets: visibleTickets.length,
    notifications: visibleNotifications.length,
    waitingJobs: visibleWaitingJobs.length,
  });

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

  const ticketById = useMemo(
    () => mapById(visibleTickets),
    [visibleTickets],
  );
  const jobById = useMemo(
    () => mapById(visibleWaitingJobs),
    [visibleWaitingJobs],
  );
  const notificationById = useMemo(
    () => mapById(view.notifications),
    [view.notifications],
  );

  return (
    <>
      <SharedNotificationInboxPanel
        attentionCount={view.attentionCount}
        filter={filter}
        onFilterChange={setFilter}
        filterCounts={filterCounts}
        query={query}
        onQueryChange={setQuery}
        error={error}
        onDismissError={clearError}
        loading={loading}
        hydrated={hydrated}
        empty={empty}
        hasVisibleItems={hasVisibleItems}
        markingAllRead={markingAllRead}
        unreadCount={unreadNotifications.length}
        onMarkAllRead={() => void markAllRead()}
        onRefresh={() => void refresh(workspacePath)}
        onClose={onClose}
        presentation={presentation}
        onRetry={() => void refresh(workspacePath)}
        tickets={visibleTickets.map((ticket) => ({
          id: ticket.id,
          ticket,
          sessionTitle: sessionTitles.get(ticket.chatId),
          busy: Boolean(pendingIds[`ticket:${ticket.id}`]),
          canOpenChat: sessionIds.has(ticket.chatId),
          canResume: backgroundJobs.some(
            (job) =>
              job.id === ticket.jobId && isWaitingTicketStatus(job.state),
          ),
          canDismiss: backgroundJobs.some(
            (job) =>
              job.id === ticket.jobId && isWaitingTicketStatus(job.state),
          ),
        }))}
        jobs={visibleWaitingJobs.map((job) => ({
          id: job.id,
          job,
          sessionTitle: sessionTitles.get(job.chatId),
          canOpenChat: sessionIds.has(job.chatId),
          busy: Boolean(pendingIds[`job:${job.id}`]),
          canDismiss: true,
        }))}
        unreadNotifications={visibleUnreadNotifications.map((notification) => ({
          id: notification.id,
          notification,
          sessionTitle: sessionTitles.get(notification.chatId),
          canOpenChat: sessionIds.has(notification.chatId),
          busy: Boolean(pendingIds[`notification:${notification.id}`]),
        }))}
        readNotifications={visibleReadNotifications.map((notification) => ({
          id: notification.id,
          notification,
          sessionTitle: sessionTitles.get(notification.chatId),
          canOpenChat: sessionIds.has(notification.chatId),
          busy: Boolean(pendingIds[`notification:${notification.id}`]),
        }))}
        allNotifications={visibleNotifications.map((notification) => ({
          id: notification.id,
          notification,
          sessionTitle: sessionTitles.get(notification.chatId),
          canOpenChat: sessionIds.has(notification.chatId),
          busy: Boolean(pendingIds[`notification:${notification.id}`]),
        }))}
        onOpenTicketChat={(id) => {
          const ticket = ticketById.get(id);
          if (ticket) openChat(ticket.chatId);
        }}
        onReplyTicket={(id, response) => {
          const ticket = ticketById.get(id);
          if (ticket) void replyToTicket(ticket.id, ticket.chatId, response);
        }}
        onDismissTicket={(id) => {
          const ticket = ticketById.get(id);
          if (!ticket) return;
          setDismissTarget({
            kind: "ticket",
            id: ticket.id,
            chatId: ticket.chatId,
            label: ticket.prompt,
          });
        }}
        onOpenJobChat={(id) => {
          const job = jobById.get(id);
          if (job) openChat(job.chatId);
        }}
        onDismissJob={(id) => {
          const job = jobById.get(id);
          if (!job) return;
          setDismissTarget({
            kind: "job",
            id: job.id,
            chatId: job.chatId,
            label: labelForInboxJob(job),
          });
        }}
        onOpenNotificationChat={(id) => {
          const notification = notificationById.get(id);
          if (notification) openNotificationChat(notification);
        }}
        onMarkNotificationSeen={(id) => {
          const notification = notificationById.get(id);
          if (notification) {
            void markSeen(notification.id, notification.chatId);
          }
        }}
        onResolveNotification={(id) => {
          const notification = notificationById.get(id);
          if (notification) {
            void resolveNotification(notification.id, notification.chatId);
          }
        }}
      />

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
