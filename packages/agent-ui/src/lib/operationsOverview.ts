/**
 * Aggregation model for Operations Overview (A6.104).
 *
 * Pure mapping from Work/Inbox port data to the shared `OperationsOverview`
 * props. Kept free of React and ports so it stays unit-testable in a node
 * environment. Until a canonical `OperationsSummary` projection exists, this
 * adapter composes the overview from the existing domain slices.
 */

import type {
  AutomationInfo,
  NotificationInfo,
  TaskRunInfo,
} from "@altai/host-contract";
import type {
  OperationsOverviewMetric,
  OperationsOverviewRow,
} from "../components/OperationsOverview.js";

export type OperationsOverviewData = {
  taskRuns: TaskRunInfo[];
  automations: AutomationInfo[];
  notifications: NotificationInfo[];
};

export type OperationsOverviewViewModel = {
  metrics: OperationsOverviewMetric[];
  attention: OperationsOverviewRow[];
  progressing: OperationsOverviewRow[];
};

export const EMPTY_OPERATIONS_DATA: OperationsOverviewData = {
  taskRuns: [],
  automations: [],
  notifications: [],
};

/** Rows per section; the overview summarizes, full lists live in Work/Inbox. */
const SECTION_LIMIT = 5;

const RUN_STATUS_LABELS: Record<TaskRunInfo["status"], string> = {
  queued: "Queued",
  running: "Working",
  succeeded: "Done",
  failed: "Failed",
  cancelled: "Stopped",
};

export function buildOperationsOverview(
  data: OperationsOverviewData,
): OperationsOverviewViewModel {
  const activeRuns = data.taskRuns.filter(
    (run) => run.status === "queued" || run.status === "running",
  );
  const failedRuns = data.taskRuns.filter((run) => run.status === "failed");
  const unseen = data.notifications.filter((item) => !item.seen);
  const enabledAutomations = data.automations.filter((item) => item.enabled);

  const metrics: OperationsOverviewMetric[] = [
    { label: "Active runs", value: String(activeRuns.length) },
    {
      label: "Needs attention",
      value: String(failedRuns.length + unseen.length),
    },
    { label: "Scheduled", value: String(enabledAutomations.length) },
  ];

  const attention: OperationsOverviewRow[] = [
    ...failedRuns.map((run) => ({
      id: `run:${run.id}`,
      title: run.title,
      statusLabel: RUN_STATUS_LABELS.failed,
      tone: "attention" as const,
    })),
    ...unseen.map((item) => ({
      id: `inbox:${item.id}`,
      title: item.title,
      statusLabel: "Unread",
      tone: "attention" as const,
      ...(item.body ? { detail: item.body } : {}),
    })),
  ].slice(0, SECTION_LIMIT);

  const progressing: OperationsOverviewRow[] = [
    ...activeRuns.map((run) => ({
      id: `run:${run.id}`,
      title: run.title,
      statusLabel: RUN_STATUS_LABELS[run.status],
    })),
    ...enabledAutomations.map((item) => ({
      id: `automation:${item.id}`,
      title: item.title,
      statusLabel: "Scheduled",
      detail: automationScheduleLabel(item),
    })),
  ].slice(0, SECTION_LIMIT);

  return { metrics, attention, progressing };
}

/**
 * Stable row-id prefixes (`run:`, `inbox:`, `automation:`) select the matching
 * Operations secondary route when the host advertises that capability.
 */
export type OverviewRowDestination = {
  view: "overview" | "work" | "runs" | "inbox";
  workHubView?: "runs" | "scheduled";
};

export type OverviewNavFlags = {
  taskRuns: boolean;
  automations: boolean;
  inbox: boolean;
};

export function destinationForOverviewRowId(
  id: string,
  flags: OverviewNavFlags,
): OverviewRowDestination | null {
  if (id.startsWith("run:")) {
    if (flags.taskRuns) {
      return { view: "runs" };
    }
    return null;
  }
  if (id.startsWith("inbox:")) {
    if (flags.inbox) {
      return { view: "inbox" };
    }
    return null;
  }
  if (id.startsWith("automation:")) {
    if (flags.automations) {
      return { view: "work", workHubView: "scheduled" };
    }
    if (flags.taskRuns) {
      return { view: "work", workHubView: "runs" };
    }
    return null;
  }
  return null;
}

/**
 * Map overview metric labels (from `buildOperationsOverview`) onto secondary
 * routes. Capability-gated: when the host cannot open the domain, return null
 * so the tile stays non-interactive.
 */
export function destinationForOverviewMetric(
  label: string,
  flags: OverviewNavFlags,
): OverviewRowDestination | null {
  switch (label) {
    case "Active runs":
      return flags.taskRuns ? { view: "runs" } : null;
    case "Needs attention":
      if (flags.taskRuns) {
        return { view: "runs" };
      }
      if (flags.inbox) {
        return { view: "inbox" };
      }
      return null;
    case "Scheduled":
      return flags.automations
        ? { view: "work", workHubView: "scheduled" }
        : null;
    default:
      return null;
  }
}

/**
 * Attach onOpen for metrics that have a valid capability-gated destination.
 */
export function withOverviewMetricNavigation(
  metrics: OperationsOverviewMetric[],
  flags: OverviewNavFlags,
  navigate: (destination: OverviewRowDestination) => void,
): Array<OperationsOverviewMetric & { onOpen?: () => void }> {
  return metrics.map((metric) => {
    const destination = destinationForOverviewMetric(metric.label, flags);
    if (!destination) {
      return metric;
    }
    return {
      ...metric,
      onOpen: () => {
        navigate(destination);
      },
    };
  });
}

/** Attach presentational `onOpen` handlers without coupling pure row data to React. */
export function withOverviewRowNavigation(
  rows: OperationsOverviewRow[],
  flags: OverviewNavFlags,
  navigate: (destination: OverviewRowDestination) => void,
): OperationsOverviewRow[] {
  return rows.map((row) => {
    const destination = destinationForOverviewRowId(row.id, flags);
    if (!destination) {
      return row;
    }
    return {
      ...row,
      onOpen: () => {
        navigate(destination);
      },
    };
  });
}

/**
 * Extract a task-run id from a progressing overview row when cancel is valid.
 * Active runs use status labels "Queued" or "Working".
 */
export function overviewActiveRunId(
  rowId: string,
  statusLabel: string,
): string | null {
  if (!rowId.startsWith("run:")) {
    return null;
  }
  if (statusLabel !== "Queued" && statusLabel !== "Working") {
    return null;
  }
  const id = rowId.slice("run:".length);
  return id.length > 0 ? id : null;
}

/**
 * Failed attention runs expose Retry; status copy is buildOperationsOverview's
 * "Failed" label.
 */
export function overviewFailedRunId(
  rowId: string,
  statusLabel: string,
): string | null {
  if (!rowId.startsWith("run:") || statusLabel !== "Failed") {
    return null;
  }
  const id = rowId.slice("run:".length);
  return id.length > 0 ? id : null;
}

/** Unread inbox attention rows expose Mark read. */
export function overviewUnreadInboxId(
  rowId: string,
  statusLabel: string,
): string | null {
  if (!rowId.startsWith("inbox:") || statusLabel !== "Unread") {
    return null;
  }
  const id = rowId.slice("inbox:".length);
  return id.length > 0 ? id : null;
}

/** Attention metric: failed runs + unseen notifications (matches overview). */
export function countOperationsAttention(
  data: OperationsOverviewData,
): number {
  const failed = data.taskRuns.filter((run) => run.status === "failed").length;
  const unseen = data.notifications.filter((item) => !item.seen).length;
  return failed + unseen;
}

function automationScheduleLabel(automation: AutomationInfo): string {
  const { schedule } = automation;
  if (schedule.kind === "once") {
    return `Once · ${schedule.at}`;
  }
  const ms = schedule.everyMs;
  if (ms % 3_600_000 === 0) {
    return `Every ${ms / 3_600_000}h`;
  }
  if (ms % 60_000 === 0) {
    return `Every ${ms / 60_000}m`;
  }
  return `Every ${Math.round(ms / 1000)}s`;
}
