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
import { cn } from "@/lib/utils";
import { useWorkspaceFolderStore } from "@/modules/workspace/folder";
import {
  Alert02Icon,
  Cancel01Icon,
  Notebook01Icon,
  Notification01Icon,
  Refresh01Icon,
  Tick02Icon,
  TickDouble01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useMemo, useState } from "react";
import type {
  AgentBackgroundJobInfo,
  AgentClarificationTicketInfo,
  AgentNotificationInfo,
} from "../lib/native";
import { useChatStore } from "../store/chatStore";
import {
  buildNotificationInboxView,
  useNotificationStore,
} from "../store/notificationStore";
import {
  AuxiliarySurface,
  SurfaceEmptyState,
  SurfaceIconAction,
  SurfaceSearch,
  SurfaceSectionHeader,
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
            <FilteredEmptyInbox filter={filter} onShowAll={() => setFilter("all")} />
          ) : (
            <div className="space-y-4">
              {(filter === "all" || filter === "attention") &&
              visibleTickets.length ? (
                <InboxSection
                  title="Paused tasks"
                  count={visibleTickets.length}
                >
                  {visibleTickets.map((ticket) => (
                    <TicketCard
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
                    <JobCard
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
                          label: labelForJob(job),
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
                    <NotificationCard
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
                    <NotificationCard
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

function InboxSection({
  title,
  count,
  children,
}: {
  title: string;
  count: number;
  children: React.ReactNode;
}) {
  return (
    <section>
      <SurfaceSectionHeader title={title} count={count} className="mb-2 px-0.5" />
      <div className="space-y-2">{children}</div>
    </section>
  );
}

function TicketCard({
  ticket,
  sessionTitle,
  busy,
  canOpenChat,
  canResume,
  canDismiss,
  onOpenChat,
  onReply,
  onDismiss,
}: {
  ticket: AgentClarificationTicketInfo;
  sessionTitle?: string;
  busy: boolean;
  canOpenChat: boolean;
  canResume: boolean;
  canDismiss: boolean;
  onOpenChat: () => void;
  onReply: (response: string) => void;
  onDismiss: () => void;
}) {
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

function NotificationCard({
  notification,
  sessionTitle,
  canOpenChat,
  busy,
  onOpenChat,
  onMarkSeen,
  onResolve,
}: {
  notification: AgentNotificationInfo;
  sessionTitle?: string;
  canOpenChat: boolean;
  busy: boolean;
  onOpenChat: () => void;
  onMarkSeen: () => void;
  onResolve: () => void;
}) {
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

function JobCard({
  job,
  sessionTitle,
  canOpenChat,
  busy,
  canDismiss,
  onOpenChat,
  onDismiss,
}: {
  job: AgentBackgroundJobInfo;
  sessionTitle?: string;
  canOpenChat: boolean;
  busy: boolean;
  canDismiss: boolean;
  onOpenChat: () => void;
  onDismiss: () => void;
}) {
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
              {labelForJob(job)}
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

function EmptyInbox() {
  return (
    <SurfaceEmptyState
      icon={Tick02Icon}
      title="You’re all caught up"
      description="Questions, review-ready results, and durable agent updates will appear here."
      className="border-0 bg-transparent"
    />
  );
}

function InboxLoadFailed({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center px-4 py-12 text-center">
      <span className="inline-flex size-9 items-center justify-center rounded-full bg-destructive/10 text-destructive">
        <HugeiconsIcon icon={Alert02Icon} size={17} strokeWidth={1.75} />
      </span>
      <h3 className="mt-3 text-[11.5px] font-medium text-foreground">
        Inbox could not be loaded
      </h3>
      <button
        type="button"
        onClick={onRetry}
        className="mt-2 rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
      >
        Try again
      </button>
    </div>
  );
}

function FilteredEmptyInbox({
  filter,
  onShowAll,
}: {
  filter: InboxFilter;
  onShowAll: () => void;
}) {
  const label =
    filter === "attention"
      ? "Nothing needs your attention"
      : "No updates to show";
  return (
    <div className="flex flex-col items-center justify-center px-4 py-12 text-center">
      <span className="inline-flex size-9 items-center justify-center rounded-full bg-muted text-muted-foreground">
        <HugeiconsIcon icon={Tick02Icon} size={17} strokeWidth={1.75} />
      </span>
      <h3 className="mt-3 text-[11.5px] font-medium text-foreground">{label}</h3>
      <button
        type="button"
        onClick={onShowAll}
        className="mt-2 rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
      >
        Show all inbox items
      </button>
    </div>
  );
}

function labelForJob(job: AgentBackgroundJobInfo): string {
  const kind = humanize(job.kind);
  return kind ? `${kind} background task` : "Background task";
}

function humanize(value: string): string {
  const normalized = value.trim().replace(/[_-]+/g, " ");
  return normalized
    ? normalized.charAt(0).toUpperCase() + normalized.slice(1)
    : "";
}

function formatRelativeTime(timestamp: number): string {
  const deltaMs = Date.now() - timestamp;
  if (!Number.isFinite(deltaMs) || deltaMs < 0) return "just now";
  const minutes = Math.floor(deltaMs / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return new Date(timestamp).toLocaleDateString();
}
