import { describe, expect, it } from "vitest";
import type { Routine } from "@altai/host-contract";
import {
  formatScheduleTime,
  projectRoutines,
  summarizeRoutines,
} from "./routinesProjection";

const NOW = 1_000_000_000_000;

function routine(overrides: Partial<Routine> = {}): Routine {
  return {
    id: "rt_1",
    status: "active",
    revision: 1,
    triggerKind: "recurring",
    cronExpression: "0 9 * * *",
    eventSource: null,
    targetWorkId: "wi_work_1",
    targetWorkTitle: "Nightly digest",
    lastFiredAtMs: NOW - 3_600_000,
    nextFireAtMs: NOW + 3_600_000,
    updatedAtMs: NOW - 60_000,
    ...overrides,
  };
}

describe("formatScheduleTime", () => {
  it("formats upcoming fires with a leading 'in'", () => {
    expect(formatScheduleTime(NOW + 90_000, NOW)).toBe("in 1m");
    expect(formatScheduleTime(NOW + 5 * 3_600_000, NOW)).toBe("in 5h");
    expect(formatScheduleTime(NOW + 3 * 86_400_000, NOW)).toBe("in 3d");
  });

  it("labels past boundaries as overdue, not 'ago'", () => {
    expect(formatScheduleTime(NOW - 30_000, NOW)).toBe("just now");
    expect(formatScheduleTime(NOW - 2 * 3_600_000, NOW)).toBe("2h overdue");
    expect(formatScheduleTime(NOW - 4 * 86_400_000, NOW)).toBe("4d overdue");
  });
});

describe("projectRoutines", () => {
  it("sorts overdue recurring routines before upcoming ones", () => {
    const rows = projectRoutines(
      [
        routine({ id: "rt-soon", nextFireAtMs: NOW + 7_200_000 }),
        routine({ id: "rt-overdue", nextFireAtMs: NOW - 3_600_000 }),
      ],
      NOW,
    );
    expect(rows.map((row) => row.id)).toEqual(["rt-overdue", "rt-soon"]);
    expect(rows[0].isOverdue).toBe(true);
    expect(rows[1].isOverdue).toBe(false);
  });

  it("places event-triggered routines after scheduled ones, newest first", () => {
    const rows = projectRoutines(
      [
        routine({
          id: "rt-event-old",
          triggerKind: "event",
          cronExpression: null,
          eventSource: "github.pull_request.opened",
          nextFireAtMs: null,
          updatedAtMs: NOW - 5_000,
        }),
        routine({
          id: "rt-event-new",
          triggerKind: "event",
          cronExpression: null,
          eventSource: "github.issue.labeled",
          nextFireAtMs: null,
          updatedAtMs: NOW - 1_000,
        }),
        routine({ id: "rt-cron", nextFireAtMs: NOW + 60_000 }),
      ],
      NOW,
    );
    expect(rows.map((row) => row.id)).toEqual([
      "rt-cron",
      "rt-event-new",
      "rt-event-old",
    ]);
    expect(rows[1].scheduleLabel).toBe("github.issue.labeled");
    expect(rows[1].isOverdue).toBe(false);
  });

  it("keeps intent-less routines last with no schedule facts", () => {
    const rows = projectRoutines(
      [
        routine({
          id: "rt-bare",
          triggerKind: "recurring",
          cronExpression: null,
          eventSource: null,
          targetWorkId: null,
          targetWorkTitle: null,
          nextFireAtMs: null,
          lastFiredAtMs: NOW - 60_000,
        }),
        routine({ id: "rt-cron", nextFireAtMs: NOW + 60_000 }),
      ],
      NOW,
    );
    expect(rows.map((row) => row.id)).toEqual(["rt-cron", "rt-bare"]);
    expect(rows[1].scheduleLabel).toBeNull();
    // A fire recorded before any intent is still shown.
    expect(rows[1].lastFiredMs).toBe(NOW - 60_000);
  });

  it("is drillable only when the target Work resolved to a title", () => {
    const rows = projectRoutines(
      [routine({ targetWorkTitle: null }), routine()],
      NOW,
    );
    expect(rows[0].drillable).toBe(false);
    expect(rows[0].targetWorkId).toBe("wi_work_1");
    expect(rows[1].drillable).toBe(true);
  });
});

describe("summarizeRoutines", () => {
  it("counts lifecycle states and overdue routines", () => {
    const rows = projectRoutines(
      [
        routine({ id: "rt-1" }),
        routine({ id: "rt-2", status: "paused" }),
        routine({ id: "rt-3", nextFireAtMs: NOW - 1_000 }),
        routine({ id: "rt-4", status: "retired" }),
      ],
      NOW,
    );
    expect(summarizeRoutines(rows)).toEqual({
      totalCount: 4,
      activeCount: 2,
      pausedCount: 1,
      overdueCount: 1,
    });
  });

  it("returns zeroed counts for an empty page", () => {
    expect(summarizeRoutines([])).toEqual({
      totalCount: 0,
      activeCount: 0,
      pausedCount: 0,
      overdueCount: 0,
    });
  });
});
