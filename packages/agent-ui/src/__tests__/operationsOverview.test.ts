import { describe, expect, it } from "vitest";
import type {
  AutomationInfo,
  NotificationInfo,
  TaskRunInfo,
} from "@altai/host-contract";
import {
  buildOperationsOverview,
  countOperationsAttention,
  destinationForOverviewRowId,
  destinationForOverviewMetric,
  overviewActiveRunId,
  overviewFailedRunId,
  overviewUnreadInboxId,
  withOverviewRowNavigation,
  withOverviewMetricNavigation,
  type OverviewNavFlags,
} from "../lib/operationsOverview.js";

function taskRun(partial: Partial<TaskRunInfo> & { id: string }): TaskRunInfo {
  return {
    title: partial.id,
    status: "running",
    createdAt: "2026-08-05T00:00:00.000Z",
    ...partial,
  };
}

function automation(
  partial: Partial<AutomationInfo> & { id: string },
): AutomationInfo {
  return {
    chatId: "chat_1",
    title: partial.id,
    prompt: "do work",
    schedule: { kind: "every", everyMs: 3_600_000 },
    enabled: true,
    ...partial,
  };
}

function notification(
  partial: Partial<NotificationInfo> & { id: string },
): NotificationInfo {
  return {
    title: partial.id,
    seen: false,
    createdAt: "2026-08-05T00:00:00.000Z",
    ...partial,
  };
}

describe("buildOperationsOverview", () => {
  it("counts active runs, attention, and scheduled automations", () => {
    const view = buildOperationsOverview({
      taskRuns: [
        taskRun({ id: "a", status: "running" }),
        taskRun({ id: "b", status: "queued" }),
        taskRun({ id: "c", status: "failed" }),
        taskRun({ id: "d", status: "succeeded" }),
      ],
      automations: [
        automation({ id: "s1" }),
        automation({ id: "s2", enabled: false }),
      ],
      notifications: [notification({ id: "n1" })],
    });
    expect(view.metrics).toEqual([
      { label: "Active runs", value: "2" },
      { label: "Needs attention", value: "2" },
      { label: "Scheduled", value: "1" },
    ]);
  });

  it("routes failed runs and unseen notifications into attention", () => {
    const view = buildOperationsOverview({
      taskRuns: [taskRun({ id: "a", status: "failed", title: "Broken run" })],
      automations: [],
      notifications: [
        notification({ id: "n1", title: "Approval", body: "needs review" }),
        notification({ id: "n2", seen: true }),
      ],
    });
    expect(view.attention).toHaveLength(2);
    expect(view.attention[0]).toMatchObject({
      id: "run:a",
      title: "Broken run",
      statusLabel: "Failed",
      tone: "attention",
    });
    expect(view.attention[1]).toMatchObject({
      id: "inbox:n1",
      title: "Approval",
      statusLabel: "Unread",
      detail: "needs review",
    });
  });

  it("lists active runs and enabled automations as progressing", () => {
    const view = buildOperationsOverview({
      taskRuns: [
        taskRun({ id: "a", status: "running", title: "Working run" }),
        taskRun({ id: "b", status: "cancelled" }),
      ],
      automations: [
        automation({ id: "s1", title: "Nightly" }),
        automation({ id: "s2", enabled: false }),
      ],
      notifications: [],
    });
    expect(view.progressing).toHaveLength(2);
    expect(view.progressing[0]).toMatchObject({
      id: "run:a",
      statusLabel: "Working",
    });
    expect(view.progressing[1]).toMatchObject({
      id: "automation:s1",
      statusLabel: "Scheduled",
      detail: "Every 1h",
    });
  });

  it("renders empty sections when no data is available", () => {
    const view = buildOperationsOverview({
      taskRuns: [],
      automations: [],
      notifications: [],
    });
    expect(view.metrics.map((metric) => metric.value)).toEqual(["0", "0", "0"]);
    expect(view.attention).toEqual([]);
    expect(view.progressing).toEqual([]);
  });

  it("labels one-shot schedules", () => {
    const view = buildOperationsOverview({
      taskRuns: [],
      automations: [
        automation({
          id: "s1",
          schedule: { kind: "once", at: "2026-08-06T09:00:00.000Z" },
        }),
      ],
      notifications: [],
    });
    expect(view.progressing[0]?.detail).toBe(
      "Once · 2026-08-06T09:00:00.000Z",
    );
  });
});

describe("destinationForOverviewRowId", () => {
  const all: OverviewNavFlags = {
    taskRuns: true,
    automations: true,
    inbox: true,
  };

  it("maps stable row id prefixes onto capability-gated routes", () => {
    expect(destinationForOverviewRowId("run:abc", all)).toEqual({
      view: "runs",
    });
    expect(destinationForOverviewRowId("inbox:n1", all)).toEqual({
      view: "inbox",
    });
    expect(destinationForOverviewRowId("automation:s1", all)).toEqual({
      view: "work",
      workHubView: "scheduled",
    });
    expect(destinationForOverviewRowId("unknown:x", all)).toBeNull();
  });

  it("returns null when the destination capability is missing", () => {
    expect(
      destinationForOverviewRowId("inbox:n1", {
        taskRuns: true,
        automations: false,
        inbox: false,
      }),
    ).toBeNull();
  });
});

describe("destinationForOverviewMetric", () => {
  const all: OverviewNavFlags = {
    taskRuns: true,
    automations: true,
    inbox: true,
  };

  it("maps metric labels onto capability-gated routes", () => {
    expect(destinationForOverviewMetric("Active runs", all)).toEqual({
      view: "runs",
    });
    expect(destinationForOverviewMetric("Needs attention", all)).toEqual({
      view: "runs",
    });
    expect(destinationForOverviewMetric("Scheduled", all)).toEqual({
      view: "work",
      workHubView: "scheduled",
    });
    expect(destinationForOverviewMetric("Unknown", all)).toBeNull();
  });

  it("falls back Needs attention to inbox when runs are unavailable", () => {
    expect(
      destinationForOverviewMetric("Needs attention", {
        taskRuns: false,
        automations: false,
        inbox: true,
      }),
    ).toEqual({ view: "inbox" });
  });
});

describe("withOverviewMetricNavigation", () => {
  it("attaches onOpen only for navigable metrics", () => {
    const visited: string[] = [];
    const metrics = withOverviewMetricNavigation(
      [
        { label: "Active runs", value: "1" },
        { label: "Scheduled", value: "2" },
      ],
      { taskRuns: true, automations: false, inbox: false },
      (destination) => {
        visited.push(destination.view);
      },
    );
    expect(typeof metrics[0]?.onOpen).toBe("function");
    expect(metrics[1]?.onOpen).toBeUndefined();
    metrics[0]?.onOpen?.();
    expect(visited).toEqual(["runs"]);
  });
});

describe("withOverviewRowNavigation", () => {
  it("attaches onOpen only for navigable rows", () => {
    const visited: string[] = [];
    const rows = withOverviewRowNavigation(
      [
        { id: "run:a", title: "A", statusLabel: "Failed" },
        { id: "inbox:n1", title: "N", statusLabel: "Unread" },
      ],
      { taskRuns: true, automations: false, inbox: false },
      (destination) => {
        visited.push(destination.view);
      },
    );
    expect(typeof rows[0]?.onOpen).toBe("function");
    expect(rows[1]?.onOpen).toBeUndefined();
    rows[0]?.onOpen?.();
    expect(visited).toEqual(["runs"]);
  });
});

describe("overviewActiveRunId", () => {
  it("returns run ids only for queued/working overview rows", () => {
    expect(overviewActiveRunId("run:abc", "Working")).toBe("abc");
    expect(overviewActiveRunId("run:abc", "Queued")).toBe("abc");
    expect(overviewActiveRunId("run:abc", "Failed")).toBeNull();
    expect(overviewActiveRunId("inbox:n1", "Working")).toBeNull();
  });
});

describe("overview attention row ids", () => {
  it("detects failed runs and unread inbox rows", () => {
    expect(overviewFailedRunId("run:x", "Failed")).toBe("x");
    expect(overviewFailedRunId("run:x", "Working")).toBeNull();
    expect(overviewUnreadInboxId("inbox:n", "Unread")).toBe("n");
    expect(overviewUnreadInboxId("inbox:n", "Failed")).toBeNull();
  });
});

describe("countOperationsAttention", () => {
  it("sums failed runs and unseen notifications", () => {
    expect(
      countOperationsAttention({
        taskRuns: [
          taskRun({ id: "a", status: "failed" }),
          taskRun({ id: "b", status: "running" }),
        ],
        automations: [],
        notifications: [
          notification({ id: "n1", seen: false }),
          notification({ id: "n2", seen: true }),
        ],
      }),
    ).toBe(2);
  });
});
