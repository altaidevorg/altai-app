/**
 * Pure notification Inbox attention model (A6.203).
 * Hosts fetch rows; package builds the attention view without I/O.
 */

const TERMINAL_JOB_STATES = new Set([
  "cancelled",
  "canceled",
  "completed",
  "dismissed",
  "done",
  "failed",
  "success",
]);

export type InboxNotificationRow = {
  chatId: string;
  kind: string;
  actionKind?: string | null;
  resolvedAtMs: number | null;
  seenAtMs: number | null;
  createdAtMs: number;
};

export type InboxBackgroundJobRow = {
  id: string;
  state: string;
  updatedAtMs: number;
};

export type InboxClarificationTicketRow = {
  chatId: string;
  jobId: string;
  status: string;
  createdAtMs: number;
};

export type NotificationInboxViewModel = {
  waitingTickets: InboxClarificationTicketRow[];
  notifications: InboxNotificationRow[];
  waitingJobs: InboxBackgroundJobRow[];
  attentionCount: number;
};

/** Descending by creation time. */
export function byNewestCreatedAt(
  a: { createdAtMs: number },
  b: { createdAtMs: number },
): number {
  return b.createdAtMs - a.createdAtMs;
}

/** True when clarification ticket is waiting for a response. */
export function isWaitingTicketStatus(status: string): boolean {
  return status.trim().toLowerCase() === "waiting";
}

/** True when a background job state reports waiting (substring match). */
export function isWaitingJobState(state: string): boolean {
  return state.trim().toLowerCase().includes("waiting");
}

/** True when a background job is in a terminal/finished state. */
export function isTerminalJobState(state: string): boolean {
  return TERMINAL_JOB_STATES.has(state.trim().toLowerCase());
}

function isLinkedTicketNotification(
  notification: InboxNotificationRow,
  waitingTicketChatIds: ReadonlySet<string>,
): boolean {
  if (!waitingTicketChatIds.has(notification.chatId)) return false;
  return (
    notification.kind === "clarification_ticket" ||
    notification.actionKind === "reply_ticket"
  );
}

/**
 * Build the Inbox render model: waiting tickets, resolved-filtered
 * notifications (without dual ticket banners), and waiting jobs.
 */
export function buildNotificationInboxView(
  notifications: readonly InboxNotificationRow[],
  backgroundJobs: readonly InboxBackgroundJobRow[],
  clarificationTickets: readonly InboxClarificationTicketRow[],
): NotificationInboxViewModel {
  const waitingTickets = clarificationTickets
    .filter((ticket) => isWaitingTicketStatus(ticket.status))
    .slice()
    .sort(byNewestCreatedAt);
  const waitingTicketChatIds = new Set(
    waitingTickets.map((ticket) => ticket.chatId),
  );
  const waitingJobIds = new Set(waitingTickets.map((ticket) => ticket.jobId));

  const visibleNotifications = notifications
    .filter((notification) => notification.resolvedAtMs === null)
    .filter(
      (notification) =>
        !isLinkedTicketNotification(notification, waitingTicketChatIds),
    )
    .slice()
    .sort(byNewestCreatedAt);

  const waitingJobs = backgroundJobs
    .filter((job) => !isTerminalJobState(job.state))
    .filter((job) => !waitingJobIds.has(job.id))
    .filter((job) => isWaitingJobState(job.state))
    .slice()
    .sort((a, b) => b.updatedAtMs - a.updatedAtMs);

  const unreadNotifications = visibleNotifications.filter(
    (notification) => notification.seenAtMs === null,
  ).length;

  return {
    waitingTickets,
    notifications: visibleNotifications,
    waitingJobs,
    attentionCount:
      waitingTickets.length + unreadNotifications + waitingJobs.length,
  };
}
