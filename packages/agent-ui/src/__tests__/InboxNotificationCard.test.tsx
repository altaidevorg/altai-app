import { createElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { InboxNotificationCard } from "../components/InboxNotificationCard.js";
import {
  formatRelativeTime,
  humanize,
} from "../lib/inboxFormat.js";

describe("inboxFormat", () => {
  it("humanizes kind labels", () => {
    expect(humanize("task_done")).toBe("Task done");
    expect(humanize("")).toBe("");
  });

  it("formats relative timestamps", () => {
    const now = 1_700_000_000_000;
    expect(formatRelativeTime(now - 30_000, now)).toBe("just now");
    expect(formatRelativeTime(now - 5 * 60_000, now)).toBe("5m ago");
    expect(formatRelativeTime(now - 3 * 60 * 60_000, now)).toBe("3h ago");
  });
});

describe("InboxNotificationCard", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2024-01-15T12:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders unread notification with mark-read action", () => {
    const html = renderToStaticMarkup(
      createElement(InboxNotificationCard, {
        notification: {
          title: "Build finished",
          body: "All checks passed",
          kind: "task_done",
          createdAtMs: Date.now() - 5 * 60_000,
          seenAtMs: null,
        },
        sessionTitle: "Fix CI",
        canOpenChat: true,
        busy: false,
        onOpenChat: () => {},
        onMarkSeen: () => {},
        onResolve: () => {},
      }),
    );
    expect(html).toContain("Build finished");
    expect(html).toContain("All checks passed");
    expect(html).toContain("Task done");
    expect(html).toContain("Fix CI");
    expect(html).toContain("5m ago");
    expect(html).toContain("Mark read");
    expect(html).toContain("Dismiss");
    expect(html).toContain("Open chat");
  });

  it("hides mark-read when already seen", () => {
    const html = renderToStaticMarkup(
      createElement(InboxNotificationCard, {
        notification: {
          title: "Seen",
          body: null,
          kind: "info",
          createdAtMs: Date.now(),
          seenAtMs: Date.now(),
        },
        canOpenChat: false,
        busy: false,
        onOpenChat: () => {},
        onMarkSeen: () => {},
        onResolve: () => {},
      }),
    );
    expect(html).not.toContain("Mark read");
    expect(html).toContain("Dismiss");
  });
});
