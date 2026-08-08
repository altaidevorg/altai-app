/**
 * Shell-owned attention polling when the Operations surface is not mounted
 * (A6.105). Pure fetch + count helpers keep the adapter unit-testable.
 */

import type { NotificationInfo, TaskRunInfo } from "@altai/host-contract";
import {
  countOperationsAttention,
  type OperationsOverviewData,
} from "./operationsOverview.js";

export type AttentionPollFlags = {
  taskRuns: boolean;
  inbox: boolean;
};

export type AttentionPollSources = {
  listTaskRuns: () => Promise<TaskRunInfo[]>;
  listNotifications: () => Promise<NotificationInfo[]>;
};

/**
 * Load Work/Inbox slices needed for the status-bar attention badge.
 * Returns 0 when neither capability is available (no transport calls).
 */
export async function fetchOperationsAttentionCount(
  flags: AttentionPollFlags,
  sources: AttentionPollSources,
): Promise<number> {
  if (!flags.taskRuns && !flags.inbox) {
    return 0;
  }
  const [taskRuns, notifications] = await Promise.all([
    flags.taskRuns ? sources.listTaskRuns() : Promise.resolve([]),
    flags.inbox ? sources.listNotifications() : Promise.resolve([]),
  ]);
  const data: OperationsOverviewData = {
    taskRuns,
    automations: [],
    notifications,
  };
  return countOperationsAttention(data);
}

/** Host events that may change failed runs or unread inbox. */
export function shouldRefreshAttentionOnEvent(type: string): boolean {
  return type === "lifecycle" || type === "notification";
}
