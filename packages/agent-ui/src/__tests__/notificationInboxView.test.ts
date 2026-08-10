import { describe, expect, it } from "vitest";
import {
  buildNotificationInboxView,
  isTerminalJobState,
  isWaitingTicketStatus,
} from "../lib/notificationInboxView.js";

describe("notificationInboxView", () => {
  it("classifies wait/terminal", () => {
    expect(isWaitingTicketStatus(" Waiting ")).toBe(true);
    expect(isTerminalJobState("completed")).toBe(true);
    expect(isTerminalJobState("running")).toBe(false);
  });

  it("builds attention count", () => {
    const view = buildNotificationInboxView(
      [
        {
          chatId: "c1",
          kind: "info",
          resolvedAtMs: null,
          seenAtMs: null,
          createdAtMs: 2,
        },
      ],
      [
        {
          id: "j1",
          state: "waiting_for_user",
          updatedAtMs: 1,
        },
      ],
      [
        {
          chatId: "c2",
          jobId: "j2",
          status: "waiting",
          createdAtMs: 3,
        },
      ],
    );
    expect(view.waitingTickets).toHaveLength(1);
    expect(view.waitingJobs).toHaveLength(1);
    expect(view.notifications).toHaveLength(1);
    expect(view.attentionCount).toBe(3);
  });

  it("hides linked ticket notifications while ticket waits", () => {
    const view = buildNotificationInboxView(
      [
        {
          chatId: "c1",
          kind: "clarification_ticket",
          resolvedAtMs: null,
          seenAtMs: null,
          createdAtMs: 1,
        },
      ],
      [],
      [
        {
          chatId: "c1",
          jobId: "j1",
          status: "waiting",
          createdAtMs: 2,
        },
      ],
    );
    expect(view.notifications).toHaveLength(0);
    expect(view.attentionCount).toBe(1);
  });
});
