/**
 * Pure Notification Inbox filter/search chrome (A6.215).
 */

export type NotificationInboxFilterId = "all" | "attention" | "updates";

export function matchesSearchFields(
  values: ReadonlyArray<string | null | undefined>,
  query: string,
): boolean {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return true;
  return values
    .filter((value): value is string => Boolean(value))
    .join("\n")
    .toLowerCase()
    .includes(normalizedQuery);
}

export type InboxFilterCountSource = {
  waitingTickets: number;
  notifications: number;
  waitingJobs: number;
  unreadNotifications: number;
};

export function notificationInboxFilterCounts(
  source: InboxFilterCountSource,
): Record<NotificationInboxFilterId, number> {
  return {
    all:
      source.waitingTickets + source.notifications + source.waitingJobs,
    attention:
      source.waitingTickets +
      source.unreadNotifications +
      source.waitingJobs,
    updates: source.notifications,
  };
}

export function notificationInboxHasVisibleItems(
  filter: NotificationInboxFilterId,
  counts: {
    tickets: number;
    notifications: number;
    waitingJobs: number;
  },
): boolean {
  const showTickets = filter === "all" || filter === "attention";
  const showNotifications =
    filter === "all" || filter === "attention" || filter === "updates";
  const showJobs = filter === "all" || filter === "attention";
  return (
    (showTickets && counts.tickets > 0) ||
    (showNotifications && counts.notifications > 0) ||
    (showJobs && counts.waitingJobs > 0)
  );
}

/** Attention filter shows unread only; otherwise show full list. */
export function notificationsForInboxFilter<T extends { seenAtMs: number | null }>(
  filter: NotificationInboxFilterId,
  all: readonly T[],
  unread: readonly T[],
): readonly T[] {
  return filter === "attention" ? unread : all;
}

export function partitionNotificationsByReadState<
  T extends { seenAtMs: number | null },
>(notifications: readonly T[]): { unread: T[]; read: T[] } {
  return {
    unread: notifications.filter((n) => n.seenAtMs === null),
    read: notifications.filter((n) => n.seenAtMs !== null),
  };
}
