import { createElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { InboxTicketCard } from "../components/InboxTicketCard.js";

describe("InboxTicketCard", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2024-01-15T12:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders resumable ticket with choices and actions", () => {
    const html = renderToStaticMarkup(
      createElement(InboxTicketCard, {
        ticket: {
          prompt: "Which model should I use?",
          choices: ["fast", "smart"],
          updatedAtMs: Date.now() - 120_000,
        },
        sessionTitle: "Nightly",
        busy: false,
        canOpenChat: true,
        canResume: true,
        canDismiss: true,
        onOpenChat: () => {},
        onReply: () => {},
        onDismiss: () => {},
      }),
    );
    expect(html).toContain("Background task is paused");
    expect(html).toContain("Which model should I use?");
    expect(html).toContain("Nightly");
    expect(html).toContain("2m ago");
    expect(html).toContain("fast");
    expect(html).toContain("smart");
    expect(html).toContain("Reply &amp; resume");
    expect(html).toContain("Dismiss waiting task");
    expect(html).toContain("Response to clarification ticket");
    expect(html).toContain("<svg");
  });

  it("shows read-only choices when resume is unavailable", () => {
    const html = renderToStaticMarkup(
      createElement(InboxTicketCard, {
        ticket: {
          prompt: "Pick one",
          choices: ["A"],
          updatedAtMs: Date.now(),
        },
        busy: false,
        canOpenChat: false,
        canResume: false,
        canDismiss: false,
        onOpenChat: () => {},
        onReply: () => {},
        onDismiss: () => {},
      }),
    );
    expect(html).toContain("Available choices");
    expect(html).toContain("no longer waiting for a reply");
    expect(html).toContain("Waiting for safe resume routing");
    expect(html).not.toContain("Reply & resume");
  });
});
