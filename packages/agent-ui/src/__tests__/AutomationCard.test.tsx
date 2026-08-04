import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  AutomationCard,
  automationLastRunLabel,
  automationNextRunLabel,
  automationScheduleLabel,
} from "../components/AutomationCard.js";

describe("automation label helpers", () => {
  it("formats once / every / cron schedules", () => {
    expect(
      automationScheduleLabel({ kind: "at", atMs: Date.parse("2026-08-04T12:00:00Z") }),
    ).toContain("Once ·");
    expect(
      automationScheduleLabel({ kind: "every", everyMs: 60 * 60_000 }),
    ).toBe("Every 1h");
    expect(
      automationScheduleLabel({ kind: "every", everyMs: 45 * 60_000 }),
    ).toBe("Every 45m");
    expect(
      automationScheduleLabel({ kind: "cron", cronExpr: "0 9 * * 1" }),
    ).toBe("Cron · 0 9 * * 1");
  });

  it("formats next/last run copy", () => {
    expect(automationLastRunLabel(null)).toBe("Not run yet");
    expect(
      automationNextRunLabel({
        schedule: { kind: "every", everyMs: 60_000 },
        lastRunAtMs: null,
      }),
    ).toBe("Next run after initial sync");
    expect(
      automationNextRunLabel({
        schedule: { kind: "cron", cronExpr: "* * * * *" },
        lastRunAtMs: 1,
      }),
    ).toBe("Next run determined by cron expression");
  });
});

describe("AutomationCard", () => {
  it("renders schedule and actions", () => {
    const html = renderToStaticMarkup(
      createElement(AutomationCard, {
        message: "Run the test suite nightly",
        scheduleLabel: "Every 1h",
        nextRunLabel: "Next soon",
        lastRunLabel: "Not run yet",
        owningChatLabel: "Ops chat",
        onOpenChat: () => {},
        onDuplicate: () => {},
        onRemove: () => {},
      }),
    );
    expect(html).toContain("Run the test suite nightly");
    expect(html).toContain("Every 1h");
    expect(html).toContain("Ops chat");
    expect(html).toContain("Duplicate automation");
    expect(html).toContain("Remove automation");
  });

  it("shows failed job state and pending remove", () => {
    const html = renderToStaticMarkup(
      createElement(AutomationCard, {
        message: "Health check",
        scheduleLabel: "Once · later",
        nextRunLabel: "Scheduled later",
        lastRunLabel: "Last run yesterday",
        owningChatLabel: "Main",
        jobError: "timeout",
        pendingRemove: true,
        onOpenChat: () => {},
        onDuplicate: () => {},
        onRemove: () => {},
      }),
    );
    expect(html).toContain("Failed: timeout");
    expect(html).toContain("disabled");
  });
});
