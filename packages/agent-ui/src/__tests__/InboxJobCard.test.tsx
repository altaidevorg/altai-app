import { createElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  InboxJobCard,
  labelForInboxJob,
} from "../components/InboxJobCard.js";

describe("InboxJobCard", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2024-01-15T12:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("labels jobs from kind", () => {
    expect(labelForInboxJob({ kind: "agent" })).toBe("Agent background task");
    expect(labelForInboxJob({ kind: "" })).toBe("Background task");
  });

  it("renders waiting job with dismiss action", () => {
    const html = renderToStaticMarkup(
      createElement(InboxJobCard, {
        job: {
          kind: "agent",
          state: "waiting_user",
          updatedAtMs: Date.now() - 60_000,
          resumeAfterRestart: true,
          detached: true,
          lastError: "Need input",
        },
        sessionTitle: "Nightly",
        canOpenChat: true,
        busy: false,
        canDismiss: true,
        onOpenChat: () => {},
        onDismiss: () => {},
      }),
    );
    expect(html).toContain("Agent background task");
    expect(html).toContain("Waiting user");
    expect(html).toContain("1m ago");
    expect(html).toContain("resumes after restart");
    expect(html).toContain("detached");
    expect(html).toContain("Need input");
    expect(html).toContain("Nightly");
    expect(html).toContain("Dismiss waiting task");
    expect(html).toContain("<svg");
  });

  it("hides dismiss when not allowed", () => {
    const html = renderToStaticMarkup(
      createElement(InboxJobCard, {
        job: {
          kind: "cron",
          state: "running",
          updatedAtMs: Date.now(),
          resumeAfterRestart: false,
          detached: false,
          lastError: null,
        },
        canOpenChat: false,
        busy: true,
        canDismiss: false,
        onOpenChat: () => {},
        onDismiss: () => {},
      }),
    );
    expect(html).not.toContain("Dismiss waiting task");
    expect(html).toContain("Running");
  });
});
