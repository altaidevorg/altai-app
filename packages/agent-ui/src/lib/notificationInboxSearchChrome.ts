/**
 * Pure Notification Inbox emptiness + row search filter (A6.236).
 */

import { matchesSearchFields } from "./notificationInboxFilterChrome.js";

/** True when inbox view has no tickets, notifications, or waiting jobs. */
export function isNotificationInboxEmpty(input: {
  waitingTickets: number;
  notifications: number;
  waitingJobs: number;
}): boolean {
  return (
    input.waitingTickets === 0 &&
    input.notifications === 0 &&
    input.waitingJobs === 0
  );
}

/**
 * Keep rows whose host-provided fields match free-text inbox search.
 */
export function filterRowsBySearchFields<T>(
  rows: readonly T[],
  query: string,
  fieldsOf: (row: T) => ReadonlyArray<string | null | undefined>,
): T[] {
  return rows.filter((row) => matchesSearchFields(fieldsOf(row), query));
}
